use std::sync::{Arc, RwLock};

// https://crates.io/crates/radiobrowser
// https://github.com/aschey/stream-download-rs
// TODO
// Figure out how to get main window id (so we dont cancel the token when closing settings or browser)
// Favicon rendering!
use iced::Length::Fixed;
use iced::widget::{
    button, column, container, horizontal_space, row, scrollable, slider, text, text_input,
    vertical_space,
};
use iced::{Element, Font, Size, Subscription, Task, Theme, window};
use tokio_util::sync::CancellationToken;

mod streamer;

#[derive(Debug, Clone)]
pub enum Error {
    Error,
}

fn main() -> iced::Result {
    // switch this to a daemon so we can create the first window, get it's id
    // and then detect when it is closed to kill the sink.
    iced::daemon("Radio", Radio::update, Radio::view)
        .default_font(Font::MONOSPACE)
        .subscription(Radio::subscription)
        .theme(Radio::theme)
        .run_with(Radio::new)
}

#[derive(Debug, Clone)]
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
                stations: vec![
                    Station {
                        name: "BBC World Service".to_string(),
                        url: "http://stream.live.vc.bbcmedia.co.uk/bbc_world_service".to_string(),
                    },
                    Station {
                        name: "24/7 LoFi".to_string(),
                        url: "http://usa9.fastcast4u.com/proxy/jamz?mp=/1".to_string(),
                    },
                ],
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

    fn theme(&self, _window: window::Id) -> Theme {
        Theme::default()
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
            Message::Stopped(_) => Task::none(),
            Message::Stop => {
                self.token.cancel();
                Task::none()
            }
            Message::VolumeChanged(new_vol) => {
                *self.volume.write().unwrap() = new_vol / 100.0;
                Task::none()
            }
            Message::AddStationDialog => {
                if self.dialog_window.is_none(){
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
                    Task::none()
                }
            }
            Message::WindowOpened(_) => Task::none(),
            Message::StationNameChanged(name) => {
                self.new_station_name = name;
                Task::none()
            }
            Message::StationUrlChanged(url) => {
                self.new_station_url = url;
                Task::none()
            }
            Message::AddNewStation => {
                self.stations.push(Station {
                    name: self.new_station_name.clone(),
                    url: self.new_station_url.clone(),
                });
                let window_close = window::close(self.dialog_window.unwrap()); // we know that if this is fired the window exists so an unwrap is fine
                self.dialog_window = None;
                window_close
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        window::close_events().map(Message::WindowClosed)
    }

    fn view(&self, window_id: window::Id) -> Element<Message> {
        if window_id == self.main_window {
            let radio_interface = column![
                station_list_element(self.stations.clone()),
                global_controls(self.volume.clone())
            ];

            radio_interface.into()
        } else {
            column![
                text("Station Name:"),
                text_input("", &self.new_station_name).on_input(Message::StationNameChanged),
                text("Station Url:"),
                text_input("", &self.new_station_url).on_input(Message::StationUrlChanged),
                button("Add").on_press(Message::AddNewStation),
            ]
            .spacing(5)
            .padding(5)
            .into()
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
    let mut station_list = column![];
    station_list = station_list
        .extend(stations.into_iter().map(|station| station_element(station)))
        .padding(5)
        .spacing(5);

    let station_scrollable = scrollable(station_list);
    column![station_scrollable, vertical_space()].into()
}

fn station_element<'a>(station: Station) -> Element<'a, Message> {
    container(row![
        text(station.name),
        horizontal_space(),
        button("Play").on_press(Message::Play(station.url)),
    ])
    .padding(10)
    .style(container::rounded_box)
    .into()
}
