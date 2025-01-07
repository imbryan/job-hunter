mod data;

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use iced::{Alignment, color, Element, Fill, Length, Padding, Subscription, Task, Theme, Vector, window};
use iced::event::Event;
use iced::keyboard;
use iced::keyboard::key;
use iced::widget::{button, center, checkbox, Column, column, container, focus_next, focus_previous, horizontal_space, mouse_area, opaque, row, scrollable, stack, text, text_input};
use iced_aw::{drop_down, DropDown, helpers::badge, number_input, style};
use iced_font_awesome::{fa_icon, fa_icon_solid};
use rusqlite::Connection;

use self::data::{Company, JobApplication, JobApplicationStatus, JobPost, migrate};

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
    careers_url: String,
    company_dropdowns: BTreeMap<i32, bool>,
    company_id: Option<i32>,
    job_posts: Vec<JobPost>,
    filter_min_yoe: i32,
    filter_max_yoe: i32,
    filter_onsite: bool,
    filter_hybrid: bool,
    filter_remote: bool,
    filter_job_title: String,
    filter_location: String,
    job_dropdowns: BTreeMap<i32, bool>,
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
    CompanyNameChanged(String),
    CareersURLChanged(String),
    ToggleCompanyDropdown(i32),
    ShowEditCompanyModal(i32),
    EditCompany,
    FilterMinYOEChanged(i32),
    FilterMaxYOEChanged(i32),
    FilterOnsiteChanged(bool),
    FilterHybridChanged(bool),
    FilterRemoteChanged(bool),
    FilterJobTitleChanged(String),
    FilterLocationChanged(String),
    ResetFilters,
    ToggleJobDropdown(i32),
}

pub struct Window {
}

impl Window {
    fn new() -> Self {
        Self {
        }
    }
}

pub fn ellipsis_button(color: iced::Color) -> iced::widget::Button<'static, Message> {
    button(fa_icon_solid("ellipsis").color(color).size(15.0))
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
        let jobs = JobPost::get_all(&conn).expect("Failed to get jobs");
        let (id, open) = window::open(window::Settings::default());
        (
            Self {
            companies: companies,
            db: conn,
            windows: BTreeMap::new(),
            main_window: id,
            modal: Modal::None,
            company_name: "".to_string(),
            careers_url: "".to_string(),
            company_dropdowns: BTreeMap::new(),
            company_id: None,
            job_posts: jobs,
            filter_min_yoe: 0,
            filter_max_yoe: 0,
            filter_onsite: false,
            filter_hybrid: false,
            filter_remote: false,
            filter_job_title: "".to_string(),
            filter_location: "".to_string(),
            job_dropdowns: BTreeMap::new(),
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

    fn company_modal<'a>(&self, submit_message: Message) -> Element<'a, Message> {
        container(
            column![
                text("Track Company").size(24),
                column![
                    column![
                        text("Company Name").size(12),
                        text_input("", &self.company_name) // hmm...
                            .on_input(Message::CompanyNameChanged)
                            .on_submit(submit_message.clone())
                            .padding(5)
                    ]
                    .spacing(5),
                    column![
                        text("Company's Careers Page URL").size(12),
                        text_input("", &self.careers_url)
                            .on_input(Message::CareersURLChanged)
                            .on_submit(submit_message.clone())
                            .padding(5)
                    ]
                    .spacing(5),
                    row![
                        container(button(text("Save")).on_press(submit_message.clone()))
                        .width(Fill)
                        .align_x(Alignment::End),
                        button(text("Cancel")).on_press(Message::HideModal)
                    ]
                    .spacing(10)
                    .width(Fill)
                ]
                .spacing(10),
            ]
            .spacing(20)
        )
        .width(300)
        .padding(10)
        .style(container::rounded_box)
        .into()
    }

    fn hide_modal(&mut self) {
        self.modal = Modal::None;
        self.company_name = "".to_string(); // hmm...
        self.careers_url = "".to_string();
        self.company_id = None;
    }

    fn reset_filters(&mut self) {
        self.filter_job_title = "".to_string();
        self.filter_location = "".to_string();
        self.filter_min_yoe = 0;
        self.filter_max_yoe = 0;
        self.filter_onsite = false;
        self.filter_hybrid = false;
        self.filter_remote = false;

        let jobs = JobPost::get_all(&self.db).expect("Failed to get job posts");
        self.job_posts = jobs;
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
                if self.company_name == "" || self.careers_url == "" { // hmm...
                    return Task::none() // TODO ideally there would be visual feedback
                }
                let _ = Company::create(&self.db, self.company_name.clone(), self.careers_url.clone());
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
            Message::CompanyNameChanged(name) => {
                self.company_name = name; // hmm...
                Task::none()
            }
            Message::CareersURLChanged(careers_url) => {
                self.careers_url = careers_url;
                Task::none()
            }
            Message::ToggleCompanyDropdown(id) => {
                let current_val = match self.company_dropdowns.get(&id) {
                    Some(&status) => status,
                    None => false
                };
                self.company_dropdowns.insert(id, !current_val);
                Task::none()
            }
            Message::ShowEditCompanyModal(id) => {
                let company = Company::get(&self.db, id).unwrap();
                self.company_name = company.name;
                self.careers_url = company.careers_url;
                self.company_id = Some(id);
                self.company_dropdowns.insert(id, false);
                self.modal = Modal::EditCompanyModal;
                focus_next()
            }
            Message::EditCompany => {
                let company_id = match self.company_id {
                    Some(id) => {
                        id
                    }
                    None => {
                        return Task::none()
                    }
                };
                if self.company_name == "" || self.careers_url == "" {
                    return Task::none() // TODO visual feedback
                }
                let company = Company {
                    id: company_id,
                    name: self.company_name.clone(),
                    careers_url: self.careers_url.clone(),
                };
                let _ = Company::update(&self.db, company).expect("Failed to update company");
                self.companies = Company::get_all(&self.db).expect("Failed to get companies");
                self.hide_modal();
                Task::none()
            }
            Message::FilterMinYOEChanged(num) => {
                self.filter_min_yoe = num;
                Task::none()
            }
            Message::FilterMaxYOEChanged(num) => {
                self.filter_max_yoe = num;
                Task::none()
            }
            Message::FilterOnsiteChanged(val) => {
                self.filter_onsite = val;
                Task::none()
            }
            Message::FilterHybridChanged(val) => {
                self.filter_hybrid = val;
                Task::none()
            }
            Message::FilterRemoteChanged(val) => {
                self.filter_remote = val;
                Task::none()
            }
            Message::FilterJobTitleChanged(title) => {
                self.filter_job_title = title;
                Task::none()
            }
            Message::FilterLocationChanged(location) => {
                self.filter_location = location;
                Task::none()
            }
            Message::ResetFilters => {
                self.reset_filters();
                Task::none()
            }
            Message::ToggleJobDropdown(id) => {
                let current_val = match self.job_dropdowns.get(&id) {
                    Some(&status) => status,
                    None => false
                };
                self.job_dropdowns.insert(id, !current_val);
                Task::none()
            }
            // Event Messages
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
                        text("Companies"),
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
                                    let company_id = company.id;
                                    let underlay = ellipsis_button(color!(255,255,255)).on_press(Message::ToggleCompanyDropdown(company_id));
                                    let dropdown = DropDown::new(
                                        underlay,
                                        column(vec![
                                            button(text("Edit"))
                                                .on_press(Message::ShowEditCompanyModal(company_id))
                                                .into(),
                                            button(text("Exclude"))
                                                .into(),
                                            button(text("Delete"))
                                                .on_press(Message::DeleteCompany(company_id))
                                                .into(),
                                        ])
                                        .spacing(5),
                                        match self.company_dropdowns.get(&company_id) {
                                            Some(&status) => status,
                                            None => false,
                                        }
                                    )
                                    .width(Fill)
                                    .alignment(drop_down::Alignment::BottomEnd)
                                    .on_dismiss(Message::ToggleCompanyDropdown(company_id));

                                    row![
                                        text(&company.name),
                                        container(dropdown)
                                        .width(Fill)
                                        .align_x(Alignment::End),
                                        
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
                    // Settings area
                    text("")
                    .height(Length::FillPortion(1))
                    .width(Fill)
                    .align_x(Alignment::Center)
                ]
            )
            .width(Length::FillPortion(1))
            .height(Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::from(color!(34,34,34))),
                ..Default::default()
            }),
            // Main content container
            container(
                column![
                    // Search and filter area
                    column![
                        row![
                            column![
                                text("Job Title").size(12),
                                text_input("", &self.filter_job_title)
                                    .on_input(Message::FilterJobTitleChanged)
                                    .padding(5)
                            ]
                            .spacing(5),
                            column![
                                text("Location").size(12),
                                text_input("", &self.filter_location)
                                    .on_input(Message::FilterLocationChanged)
                                    .padding(5)
                            ]
                            .spacing(5)
                        ]
                        .spacing(10),
                        row![
                            column![
                                text("Min. Years").size(12),
                                number_input(self.filter_min_yoe, 0..100, Message::FilterMinYOEChanged)
                                    .padding(5)
                                    .style(number_input::number_input::primary)
                            ]
                            .width(Length::FillPortion(1))
                            .spacing(5),
                            column![
                                text("Max. Years").size(12),
                                number_input(self.filter_max_yoe, 0..100, Message::FilterMaxYOEChanged)
                                    .padding(5)
                                    .style(number_input::number_input::primary)
                            ]
                            .width(Length::FillPortion(1))
                            .spacing(5),
                            row![
                                checkbox("On-site", self.filter_onsite)
                                    .on_toggle(Message::FilterOnsiteChanged)
                                    .width(Fill),
                                checkbox("Hybrid", self.filter_hybrid)
                                    .on_toggle(Message::FilterHybridChanged)
                                    .width(Fill),
                                checkbox("Remote", self.filter_remote)
                                    .on_toggle(Message::FilterRemoteChanged)
                                    .width(Fill),
                            ]
                            .width(Length::FillPortion(2))
                            .spacing(25),
                        ]
                        .spacing(10),
                        row![
                            container(
                                button(
                                    row![
                                        text("Reset"),
                                        fa_icon_solid("filter-circle-xmark").size(15.0).color(color!(255,255,255)),
                                    ]
                                    .spacing(5)
                                    .align_y(Alignment::Center)
                                ).on_press(Message::ResetFilters)
                            )
                                .width(Fill)
                                .align_x(Alignment::End),
                            button(
                                row![
                                    text("Filter Results"),
                                    fa_icon_solid("filter").size(15.0).color(color!(255,255,255)),
                                ]
                                .spacing(5)
                                .align_y(Alignment::Center)
                            ),
                            button(
                                row![
                                    text("Find Jobs"),
                                    fa_icon_solid("magnifying-glass").size(15.0).color(color!(255,255,255)),
                                ]
                                .spacing(5)
                                .align_y(Alignment::Center)
                            ),
                        ]
                        .spacing(10)
                        .width(Fill)
                    ]
                    .spacing(10)
                    .width(Fill)
                    .padding(Padding::from([0, 30]).top(20)),
                    // Job list
                    scrollable(
                        Column::with_children(
                            self.job_posts
                                .iter()
                                .map(|job_post| {
                                    let company = Company::get(&self.db, job_post.company_id).unwrap();
                                    let location_text = format!("{} ({})", &job_post.location, &job_post.location_type);
                                    let posted_text = format!("{}", &job_post.date_posted.unwrap().format("%m/%d/%Y"));

                                    let min_yoe = &job_post.min_yoe.unwrap_or(-1);
                                    let max_yoe = &job_post.max_yoe.unwrap_or(-1);
                                    let yoe_text = match (*max_yoe > -1, *min_yoe > -1) {
                                        (true, true) => format!("{}-{} years", min_yoe, max_yoe),
                                        (false, true) => format!("{}+ years", min_yoe),
                                        _ => "No required years found".to_string(),
                                    };

                                    let min_pay = &job_post.min_pay_cents.unwrap_or(-1);
                                    let max_pay = &job_post.max_pay_cents.unwrap_or(-1);
                                    let pay_text = match (*max_pay > -1, *min_pay > -1) {
                                        (true, true) => format!("${} - ${}", min_pay, max_pay),
                                        (false, true) => format!("${}", min_pay),
                                        (true, false) => format!("${}", max_pay),
                                        _ => "No salary information".to_string(),
                                    };

                                    let app_sql = "SELECT id FROM job_application WHERE job_post_id = ?";
                                    let app_id: Option<i32> = self.db.prepare(app_sql)
                                        .unwrap()
                                        .query_row([job_post.id], |row| {
                                            row.get(0)
                                        }).unwrap_or(None);
                                    let application: JobApplication;
                                    application = match app_id {
                                        Some(id) => JobApplication::get(&self.db, id).unwrap(),
                                        None => JobApplication {
                                            id: -1,
                                            job_post_id: job_post.id,
                                            status: JobApplicationStatus::New,
                                            date_applied: None,
                                            date_responded: None,
                                        },
                                    };
                                    let status_text = format!("{}", application.status);
                                    let status_style = match application.status {
                                        JobApplicationStatus::New => style::badge::info,
                                        JobApplicationStatus::Applied => style::badge::warning,
                                        JobApplicationStatus::Interview => style::badge::primary,
                                        JobApplicationStatus::Offer => style::badge::success,
                                        JobApplicationStatus::Closed => style::badge::danger,
                                        JobApplicationStatus::Rejected => style::badge::danger,
                                    };

                                    // Dropdown
                                    let underlay = ellipsis_button(color!(255,255,255)).on_press(Message::ToggleJobDropdown(job_post.id));
                                    let apply_text = match app_id {
                                        Some(_) => "Application",
                                        None => "Apply",
                                    };
                                    let dropdown = DropDown::new(
                                        underlay,
                                        column(vec![
                                            button(text(apply_text))
                                                .into(),
                                            button(text("Edit"))
                                                .into(),
                                            button(text("Delete"))
                                                .into(),
                                        ])
                                        .spacing(5),
                                        match self.job_dropdowns.get(&job_post.id) {
                                            Some(&status) => status,
                                            None => false,
                                        }
                                    )
                                    .width(Fill)
                                    .alignment(drop_down::Alignment::BottomStart)
                                    .on_dismiss(Message::ToggleJobDropdown(job_post.id));
                                    
                                    container(
                                        row![
                                            column![
                                                text(&job_post.job_title),
                                                text(company.name).size(12),
                                                text(location_text).size(12),
                                                text(posted_text).size(12),
                                            ]
                                                .spacing(5)
                                                .width(Length::FillPortion(2)),
                                            column![
                                                text("Qualifications").size(12),
                                                text(yoe_text),
                                                text("Skills"),
                                            ]
                                                .spacing(5)
                                                .width(Length::FillPortion(2)),
                                            column![
                                                text("Compensation").size(12),
                                                text(pay_text),
                                                text("Benefits"),
                                            ]
                                                .spacing(5)
                                                .width(Length::FillPortion(2)),
                                            column![
                                                text("Status").size(12),
                                                badge(text(status_text)).style(status_style),
                                            ]
                                                .spacing(5)
                                                .width(Length::FillPortion(1)),
                                            row![
                                                container(dropdown)
                                                    .center_x(Fill),
                                            ],
                                        ]
                                        .width(Fill)
                                    )
                                    .padding(Padding::from(10))
                                    .style(|_| container::Style {
                                        background: Some(iced::Background::from(color!(34,34,34))),
                                        ..container::rounded_box(&self.theme(self.main_window))
                                    })
                                    .into()
                                })
                        )
                            .spacing(15)
                            .padding(Padding::from([20, 30]))
                    )
                ]
                .spacing(15)
            )
            .width(Length::FillPortion(3))
            .height(Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::from(color!(20,20,20))),
                ..Default::default()
            })
        ];

        match self.modal {
            Modal::CreateCompanyModal => {
                let create_company_content = self.company_modal(Message::TrackNewCompany);

                modal(main_window_content, create_company_content, Message::HideModal)
            }
            Modal::EditCompanyModal => {
                let edit_company_content = self.company_modal(Message::EditCompany);

                modal(main_window_content, edit_company_content, Message::HideModal)
            }
            Modal::None => {
                main_window_content.into()
            }
        }
    }
    
}
