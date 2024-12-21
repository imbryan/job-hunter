mod data;

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use iced::{Alignment, color, Element, Fill, Length, Padding, Subscription, Task, Theme, Vector, window};
use iced::event::Event;
use iced::keyboard;
use iced::keyboard::key;
use iced::widget::{button, center, Column, column, container, focus_next, focus_previous, horizontal_space, mouse_area, opaque, row, scrollable, stack, text, text_input};
use iced_font_awesome::{fa_icon, fa_icon_solid};
use rusqlite::Connection;

use self::data::{Company, migrate};

pub fn main() -> iced::Result {
    iced::daemon(JobHunter::title, JobHunter::update, JobHunter::view)
        .theme(JobHunter::theme)
        .subscription(JobHunter::subscription)
        .run_with(JobHunter::new)
}

pub struct JobHunter {
    companies: Vec<Company>,
    db: Connection,
    windows: BTreeMap<window::Id, Window>,
    main_window: window::Id,
    modal: Modal,
    company_name: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleCompanyMenu,
    TrackNewCompany,
    DeleteCompany(i32),
    WindowOpened(window::Id), 
    WindowClosed(window::Id),
    OpenWindow, 
    ShowCreateCompanyModal,
    HideModal,
    Event(Event),
    CompanyName(String),
}

pub struct Window {
}

impl Window {
    fn new() -> Self {
        Self {
        }
    }
}

pub fn ellipsis_button(message: Message, color: iced::Color) -> iced::widget::Button<'static, Message> {
    button(fa_icon_solid("ellipsis").color(color).size(15.0)).on_press(message)
}

pub enum Modal {
    None,
    CreateCompanyModal,
    EditCompanyModal,
}

// https://github.com/iced-rs/iced/blob/latest/examples/modal/src/main.rs
fn modal<'a, Message>(
    base: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
    on_blur: Message,
) -> Element<'a, Message> where Message: Clone + 'a, {
    stack![
        base.into(),
        opaque(
            mouse_area(center(opaque(content)).style(|_theme| {
                container::Style {
                    background: Some(
                        iced::Color {
                            a: 0.8,
                            ..iced::Color::BLACK
                        }
                        .into(),
                    ),
                    ..container::Style::default()
                }
            }))
            .on_press(on_blur)
        )
    ]
    .into()
}

impl JobHunter {
    fn new() -> (Self, Task<Message>) {
        let mut conn = data::connect();
        migrate(&mut conn);

        let companies = Company::get_all(&conn).expect("Failed to get companies");
        let (id, open) = window::open(window::Settings::default());
        (
            Self {
            companies: companies,
            db: conn,
            windows: BTreeMap::new(),
            main_window: id,
            modal: Modal::None,
            company_name: "".to_string(),
            },
            open.map(Message::WindowOpened)
        )
    }

    fn title(&self, id: window::Id) -> String {
        String::from("Job Hunter")
    }
    
    fn theme(&self, id: window::Id) -> Theme {
        Theme::default()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            window::close_events().map(Message::WindowClosed),
            iced::event::listen().map(Message::Event),
        ])
    }

    fn hide_modal(&mut self) {
        self.modal = Modal::None;
        self.company_name = "".to_string(); // hmm...
    }
    
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenWindow => { 
                let Some(last_window) = self.windows.keys().last() else {
                    return Task::none()
                };

                window::get_position(*last_window)
                    .then(|last_position| {
                        let position = last_position.map_or(
                            window::Position::Default,
                            |last_position| {
                                window::Position::Specific(
                                    last_position + Vector::new(20.0, 20.0),
                                )
                            },
                        );

                        let (_id, open) = window::open(window::Settings {
                            position,
                            ..window::Settings::default()
                        });
                        open
                    })
                    .map(Message::WindowOpened)
            }
            Message::WindowOpened(id) => { 
                let window = Window::new();
                let focus_input = text_input::focus(format!("input-{id}")); // ?
                self.windows.insert(id, window);
                focus_input
            }
            Message::WindowClosed(id) => {
                self.windows.remove(&id);

                if self.windows.is_empty() || self.main_window == id {
                    iced::exit()
                } else {
                    Task::none()
                }
            }
            Message::TrackNewCompany => {
                if self.company_name == "" { // hmm...
                    return Task::none()
                }
                let _ = Company::create(&self.db, self.company_name.clone(), "".to_string());
                self.companies = Company::get_all(&self.db).expect("Failed to get companies");
                self.hide_modal();
                Task::none()
            }
            Message::ToggleCompanyMenu => {
                println!("Toggle menu");
                Task::none()
            }
            Message::DeleteCompany(id) => {
                let _ = Company::delete(&self.db, id);
                self.companies = Company::get_all(&self.db).expect("Failed to get companies");
                Task::none()
            }
            Message::ShowCreateCompanyModal => {
                self.modal = Modal::CreateCompanyModal;
                focus_next()
            }
            Message::HideModal => {
                self.hide_modal();
                Task::none()
            }
            Message::CompanyName(name) => {
                self.company_name = name; // hmm...
                Task::none()
            }
            Message::Event(event) => match event {
                Event::Keyboard(keyboard::Event::KeyPressed { key: keyboard::Key::Named(key::Named::Tab), 
                    modifiers,
                    ..
                }) => {
                    if modifiers.shift() {
                        focus_previous()
                    } else {
                        focus_next()
                    }
                }
                Event::Keyboard(keyboard::Event::KeyPressed { key: keyboard::Key::Named(key::Named::Escape),
                    ..
                }) => {
                    self.hide_modal();
                    Task::none()
                }
                _ => {
                    Task::none()
                }
            }
            _ => {
                println!("WARNING: undefined Message");
                Task::none()
            }
        }
    }
    
    fn view(&self, id: window::Id) -> Element<Message> {
        let main_window_content = row![
            // Sidemenu container
            container(
                column![
                    row![
                        text("All"),
                        container(
                            button(
                                row![
                                    text("New"),
                                    fa_icon_solid("plus").size(15.0).color(color!(255, 255, 255)),
                                ]
                                .spacing(5)
                                .align_y(Alignment::Center)
                            )
                            .on_press(Message::ShowCreateCompanyModal)
                        )
                        .width(Fill)
                        .align_x(Alignment::End)
                    ]
                    .align_y(iced::Alignment::Center)
                    .padding(Padding::from([20, 30]).top(30))
                    .width(Fill)
                    ,
                    scrollable(
                        Column::with_children(
                            self.companies
                                .iter()
                                .map(|company| {
                                    row![
                                        text(&company.name),
                                        container(
                                            ellipsis_button(Message::DeleteCompany(company.id), color!(255,255,255)) // TODO this needs to open menu and not just delete
                                        )
                                        .width(Fill)
                                        .align_x(Alignment::End)
                                    ]
                                    .align_y(Alignment::Center)
                                    .padding(Padding::from([5, 30]))
                                    .width(Fill)
                                    .into()
                                })
                        )
                        .spacing(5)
                        
                    )
                    .width(Fill)
                    .height(Length::FillPortion(3))
                    ,
                    text("Settings area")
                    .height(Length::FillPortion(1))
                    .width(Fill)
                    .align_x(Alignment::Center)
                ]
            )
            .width(Length::FillPortion(1))
            .height(Fill),
            // Main content container
            container(
                column![
                    text("Search and filter area")
                    .width(Fill)
                    .align_x(Alignment::Center),
                    scrollable(
                        column![
                            text("Main Content")
                            .width(Fill)
                            .align_x(Alignment::Center)
                        ]
                    )
                ]
            )
            .width(Length::FillPortion(3))
            .height(Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::from(iced::Color::BLACK)),
                ..Default::default()
            })
        ];

        match self.modal {
            Modal::CreateCompanyModal => {
                let create_company_content = container(
                    column![
                        text("Track Company").size(24),
                        column![
                            column![
                                text("Company Name").size(12),
                                text_input("", &self.company_name) // hmm...
                                    .on_input(Message::CompanyName)
                                    .on_submit(Message::TrackNewCompany)
                                    .padding(5)
                            ]
                            .spacing(5),
                            column![
                                text("Company's Careers Page URL").size(12),
                                text_input("", "") // TODO
                                    .on_submit(Message::TrackNewCompany)
                                    .padding(5)
                            ]
                            .spacing(5),
                            button(text("Save")).on_press(Message::TrackNewCompany),
                        ]
                        .spacing(10),
                    ]
                    .spacing(20)
                )
                .width(300)
                .padding(10)
                .style(container::rounded_box);

                modal(main_window_content, create_company_content, Message::HideModal)
            }
            Modal::EditCompanyModal => {
                main_window_content.into() // TODO
            }
            Modal::None => {
                main_window_content.into()
            }
        }
    }
    
}
