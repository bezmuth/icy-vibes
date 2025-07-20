use std::sync::{Arc, RwLock};

// TODO:
// Favicon rendering!
// Error dialog when a stream fails (lets actually use that result)
use iced::Length::{self, Fixed};
use iced::widget::{
    button, column, container, horizontal_space, mouse_area, row, scrollable, slider, text,
    text_input, vertical_space,
};
use iced::{Element, Size, Subscription, Task, Theme, window};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

mod saver;
mod streamer;

#[derive(Debug, Clone)]
pub enum Error {
    Error,
}

fn main() -> iced::Result {
    iced::daemon("Radio", Radio::update, Radio::view)
        .subscription(Radio::subscription)
        .theme(|_, _| Theme::default())
        .run_with(Radio::new)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Station {
    name: String,
    url: String, // https://www.radio-browser.info/
}

#[derive(Debug, Clone)]
struct Radio {
    stations: Vec<Station>,
    volume: Arc<RwLock<f32>>,
    token: CancellationToken,
    main_window: window::Id,
    dialog_window: Option<window::Id>,
    new_station_name: String,
    new_station_url: String,
}

#[derive(Debug, Clone)]
enum Message {
    Play(String),
    Stop,
    Stopped(Result<(), Error>),
    VolumeChanged(f32),
    AddStationDialog,
    WindowOpened(window::Id),
    WindowClosed(window::Id),
    StationNameChanged(String),
    StationUrlChanged(String),
    AddNewStation,
    DeleteStation(usize),
}

impl Radio {
    fn new() -> (Self, Task<Message>) {
        let (first_id, open) = window::open(window::Settings {
            size: Size::new(400.0, 400.0),
            position: window::Position::Centered,
            ..window::Settings::default()
        });
        (
            Self {
                stations: saver::load_stations(),
                // "BBC World Service"
                // "http://stream.live.vc.bbcmedia.co.uk/bbc_world_service"
                //
                // "24/7 LoFi",
                // "http://usa9.fastcast4u.com/proxy/jamz?mp=/1"
                volume: Arc::new(RwLock::new(1.0)),
                token: CancellationToken::new(),
                main_window: first_id,
                new_station_name: String::new(),
                new_station_url: String::new(),
                dialog_window: None,
            },
            open.map(Message::WindowOpened),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Play(url) => {
                self.token.cancel();
                self.token = CancellationToken::new();
                Task::perform(
                    streamer::play(url, self.volume.clone(), self.token.clone()),
                    Message::Stopped,
                )
            }
            Message::Stopped(_result) => Task::none(),
            Message::Stop => {
                self.token.cancel();
                Task::none()
            }
            Message::VolumeChanged(new_vol) => {
                *self.volume.write().unwrap() = new_vol / 100.0;
                Task::none()
            }
            Message::AddStationDialog => {
                if self.dialog_window.is_none() {
                    let (id, open) = window::open(window::Settings {
                        size: Size::new(400.0, 200.0),
                        position: window::Position::Centered,
                        ..window::Settings::default()
                    });
                    self.dialog_window = Some(id);
                    open.map(Message::WindowOpened)
                } else {
                    Task::none()
                }
            }
            Message::WindowClosed(id) => {
                if id == self.main_window {
                    self.token.cancel();
                    iced::exit()
                } else {
                    self.new_station_name = String::new();
                    self.new_station_url = String::new();
                    self.dialog_window = None;
                    Task::none()
                }
            }
            Message::WindowOpened(_id) => Task::none(),
            Message::StationNameChanged(name) => {
                self.new_station_name = name;
                Task::none()
            }
            Message::StationUrlChanged(url) => {
                self.new_station_url = url;
                Task::none()
            }
            Message::AddNewStation => {
                if (self.new_station_name == "") | (self.new_station_url == "") {
                    Task::none()
                } else {
                    self.stations.push(Station {
                        name: self.new_station_name.clone(),
                        url: self.new_station_url.clone(),
                    });
                    let window_close = window::close(self.dialog_window.unwrap()); // we know that if this is fired the window exists so an unwrap is fine
                    self.dialog_window = None;
                    saver::save_stations(self.stations.clone()).unwrap();
                    window_close
                }
            }
            Message::DeleteStation(index) => {
                self.stations.remove(index);
                saver::save_stations(self.stations.clone()).unwrap();
                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        window::close_events().map(Message::WindowClosed)
    }

    fn view(&self, window_id: window::Id) -> Element<Message> {
        if window_id == self.main_window {
            // the radio playing interface
            let radio_interface = column![
                station_list_element(self.stations.clone()),
                global_controls(self.volume.clone())
            ];

            radio_interface.into()
        } else if let Some(dialog_id) = self.dialog_window
            && dialog_id == window_id
        {
            // Station adding interface
            column![
                text("Station Name:"),
                text_input("", &self.new_station_name).on_input(Message::StationNameChanged),
                text("Station Url:"),
                text_input("", &self.new_station_url).on_input(Message::StationUrlChanged),
                vertical_space(),
                button("Add").on_press(Message::AddNewStation),
            ]
            .spacing(5)
            .padding(5)
            .into()
        } else {
            horizontal_space().into()
        }
    }
}

fn global_controls<'a>(volume: Arc<RwLock<f32>>) -> Element<'a, Message> {
    let global_controls = container(
        row![
            button("Stop").on_press(Message::Stop),
            container(slider(
                0.0..=100.0,
                *volume.read().unwrap() * 100.0,
                Message::VolumeChanged
            ))
            .center_y(Fixed(32.0))
            .width(150.0),
            horizontal_space(),
            button("Add").on_press(Message::AddStationDialog),
        ]
        .spacing(5),
    )
    .style(container::rounded_box)
    .width(iced::Length::Fill)
    .height(Fixed(54.0))
    .padding(10);

    global_controls.into()
}

fn station_list_element<'a>(stations: Vec<Station>) -> Element<'a, Message> {
    if stations.is_empty() {
        container(text("Please add some stations"))
            .center(Length::Fill)
            .into()
    } else {
        let mut station_list = column![];
        station_list = station_list
            .extend(stations.into_iter().enumerate().map(station_element))
            .padding(5)
            .spacing(5);

        let station_scrollable = scrollable(station_list);
        column![station_scrollable, vertical_space()].into()
    }
}

fn station_element<'a>(tup: (usize, Station)) -> Element<'a, Message> {
    let (index, station) = tup;
    mouse_area(
        container(row![
            text(station.name),
            horizontal_space(),
            button("Delete")
                .style(button::danger)
                .on_press(Message::DeleteStation(index))
        ])
        .padding(10)
        .style(container::rounded_box),
    )
    .on_press(Message::Play(station.url))
    .into()
}
