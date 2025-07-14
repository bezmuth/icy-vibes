// https://crates.io/crates/radiobrowser
// https://github.com/aschey/stream-download-rs
// TODO
// Figure out how to get main window id (so we dont cancel the token when closing settings or browser)
// Favicon rendering!
use iced::widget::{
    button, column, container, horizontal_space, row, scrollable, text, vertical_space,
};
use iced::window::Id;
use iced::{window, Element, Font, Length, Size, Subscription, Task};
use tokio_util::sync::CancellationToken;

mod streamer;

#[derive(Debug, Clone)]
pub enum Error {
    Error,
}

fn main() -> iced::Result {
    iced::application("Radio", Radio::update, Radio::view)
        .subscription(Radio::subscription)
        .default_font(Font::MONOSPACE)
        .centered()
        .window_size(Size::new(400.0,400.0))
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
    token: CancellationToken,
    main_window: Option<window::Id>
}

#[derive(Debug, Clone)]
enum Message {
    Play(String),
    Stop,
    Playing(Result<(), Error>),
    FirstWindow(Option<window::Id>),
    WindowClosed(window::Id),
}

impl Radio {
    fn new() -> (Self, Task<Message>) {
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
                token: CancellationToken::new(),
                main_window: None,
            },
            Task::none()
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Play(url) => {
                self.token.cancel();
                self.token = CancellationToken::new();
                Task::perform(streamer::play(url, self.token.clone()), Message::Playing)
            }
            Message::Playing(_) => Task::none(),
            Message::Stop => {
                self.token.cancel();
                self.token = CancellationToken::new();
                Task::none()
            }
            Message::FirstWindow(id) => {
                self.main_window = id;
                Task::none()
            }
            Message::WindowClosed(id) => {
                self.token.cancel();
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {

        let global_controls = row![
            horizontal_space(),
            button("Stop").on_press(Message::Stop),
            horizontal_space()
        ].padding(5);

        let radio_interface = column![station_list_element(self.stations.clone()), global_controls];

        return radio_interface.into();
    }

    fn subscription(&self) -> Subscription<Message> {
        window::close_events().map(Message::WindowClosed)
    }
}

fn station_list_element<'a>(stations: Vec<Station>) -> Element<'a, Message>{
    let mut station_list = column![];
    station_list = station_list
        .extend(
            stations
                .into_iter()
                .map(|station| station_element(station)),
        )
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
