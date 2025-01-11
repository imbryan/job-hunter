mod data;

use std::collections::BTreeMap;

use chrono::{Datelike, DateTime, NaiveDate, Utc};
use iced::{Alignment, color, Element, Fill, Font, Length, Padding, Subscription, Task, Theme, Vector, window};
use iced::event::Event;
use iced::keyboard;
use iced::keyboard::key;
use iced::widget::{button, center, checkbox, Column, column, container, focus_next, focus_previous, horizontal_space, mouse_area, opaque, row, scrollable, stack, text, text_input};
use iced_aw::{date_picker::Date, drop_down, DropDown, helpers::badge, date_picker, number_input, SelectionList, style};
use iced_font_awesome::{fa_icon, fa_icon_solid};
use rusqlite::Connection;

use self::data::{Company, format_comma_separated, get_iced_date, get_pay_i64, get_pay_str, get_utc, JobApplication, JobApplicationStatus, JobPost, JobPostLocationType, migrate};

pub fn main() -> iced::Result {
    iced::daemon(JobHunter::title, JobHunter::update, JobHunter::view)
        .theme(JobHunter::theme)
        .subscription(JobHunter::subscription)
        .run_with(JobHunter::new)
}

pub struct JobHunter {
    // Window
    windows: BTreeMap<window::Id, Window>,
    main_window: window::Id,
    // Databse
    db: Connection,
    // Company
    companies: Vec<Company>,
    company_dropdowns: BTreeMap<i32, bool>,
    // JobPosts
    job_posts: Vec<JobPost>,
    job_dropdowns: BTreeMap<i32, bool>,
    // Filter
    filter_min_yoe: i32,
    filter_max_yoe: i32,
    filter_onsite: bool,
    filter_hybrid: bool,
    filter_remote: bool,
    filter_job_title: String,
    filter_location: String,
    // Modal
    modal: Modal,
    company_name: String,
    careers_url: String,
    company_id: Option<i32>,
    job_post_id: Option<i32>,
    job_app_id: Option<i32>,
    job_app_status: Option<JobApplicationStatus>,
    job_app_status_index: Option<usize>,
    job_app_applied: Option<Date>,
    pick_job_app_applied: bool,
    job_app_responded: Option<Date>,
    pick_job_app_responded: bool,
    job_title: String,
    min_yoe: Option<i32>,
    max_yoe: Option<i32>,
    min_pay: String,
    max_pay: String,
    benefits: String,
    location: String,
    job_posted: Option<Date>,
    pick_job_posted: bool,
    location_type: Option<JobPostLocationType>,
    location_type_index: Option<usize>,
    url: String,
    skills: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    // Window
    OpenWindow, 
    WindowOpened(window::Id), 
    WindowClosed(window::Id),
    // Event
    Event(Event),
    // Company
    DeleteCompany(i32),
    TrackNewCompany,
    EditCompany,
    ToggleCompanyMenu,
    // JobApplication
    CreateApplication,
    EditApplication,
    // JobPost
    DeleteJobPost(i32),
    EditJobPost,
    // Dropdown
    ToggleCompanyDropdown(i32),
    ToggleJobDropdown(i32),
    // Filter
    ResetFilters,
    FilterMinYOEChanged(i32),
    FilterMaxYOEChanged(i32),
    FilterOnsiteChanged(bool),
    FilterHybridChanged(bool),
    FilterRemoteChanged(bool),
    FilterJobTitleChanged(String),
    FilterLocationChanged(String),
    // Modal
    HideModal,
    ShowCreateCompanyModal,
    ShowEditCompanyModal(i32),
    CompanyNameChanged(String),
    CareersURLChanged(String),
    ShowCreateApplicationModal(i32),
    ShowEditApplicationModal(i32),
    JobApplicationStatusChanged(usize, JobApplicationStatus),
    JobApplicationAppliedChanged(Date),
    JobApplicationRespondedChanged(Date),
    PickJobApplicationApplied,
    PickJobApplicationResponded,
    CancelJobApplicationPickers,
    ShowEditJobPostModal(i32),
    JobTitleChanged(String),
    MinYOEChanged(String),
    MaxYOEChanged(String),
    MinPayChanged(String),
    MaxPayChanged(String),
    BenefitsChanged(String),
    LocationChanged(String),
    PickJobPosted,
    JobPostedChanged(Date),
    CancelJobPostedPicker,
    LocationTypeChanged(usize, JobPostLocationType),
    JobURLChanged(String),
    SkillsChanged(String),
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
    CreateApplicationModal,
    EditApplicationModal,
    CreateJobPostModal,
    EditJobPostModal,
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
                job_post_id: None,
                job_app_id: None,
                job_app_status: None,
                job_app_status_index: None,
                job_app_applied: None,
                pick_job_app_applied: false,
                job_app_responded: None,
                pick_job_app_responded: false,
                job_title: "".to_string(),
                min_pay: "".to_string(),
                max_pay: "".to_string(),
                min_yoe: None,
                max_yoe: None,
                benefits: "".to_string(),
                location: "".to_string(),
                job_posted: None,
                pick_job_posted: false,
                location_type: None,
                location_type_index: None,
                skills: "".to_string(),
                url: "".to_string(),
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

    fn job_app_modal<'a>(&self, submit_message: Message) -> Element<'a, Message> {
        let title = match &self.job_app_id {
            Some(_) => "Edit Application",
            None => "New Application"
        };

        let job_status_select: SelectionList<'_, JobApplicationStatus, Message, Theme, iced::Renderer> = SelectionList::new_with(
            &JobApplicationStatus::ALL,
            Message::JobApplicationStatusChanged,
            12.0,
            5.0,
            style::selection_list::primary,
            self.job_app_status_index,
            Font::default(),
        )
            // .width(Length::Shrink)
            .height(Length::Fixed(135.0));

        let applied_btn: iced::widget::Button<'_, Message, Theme, iced::Renderer> = button(text("Pick")).on_press(Message::PickJobApplicationApplied);
        let date_applied_picker = date_picker(
            self.pick_job_app_applied,
            self.job_app_applied.unwrap_or(Date::today()),
            applied_btn,
            Message::CancelJobApplicationPickers,
            Message::JobApplicationAppliedChanged,
        );
        let applied = match &self.job_app_applied {
            Some(date) => format!("{}/{}/{}", date.month, date.day, date.year),
            None => "None".to_string(),
        };

        let responded_btn: iced::widget::Button<'_, Message, Theme, iced::Renderer> = button(text("Pick")).on_press(Message::PickJobApplicationResponded);
        let date_responded_picker = date_picker(
            self.pick_job_app_responded,
            self.job_app_responded.unwrap_or(Date::today()),
            responded_btn,
            Message::CancelJobApplicationPickers,
            Message::JobApplicationRespondedChanged,
        );
        let responded = match &self.job_app_responded {
            Some(date) => format!("{}/{}/{}", date.month, date.day, date.year),
            None => "None".to_string(),
        };

        container(
            column![
                text(title).size(24),
                column![
                    row![
                        column![
                            text("Date Applied").size(12),
                            row![
                                text(applied),
                                date_applied_picker,
                            ]
                                .spacing(10)
                                .align_y(Alignment::Center),
                        ]
                            .width(Length::FillPortion(1))
                            .spacing(5),
                        column![
                            text("Date Responded").size(12),
                            row![
                                text(responded),
                                date_responded_picker,
                            ]
                                .spacing(10)
                                .align_y(Alignment::Center),
                        ]
                            .width(Length::FillPortion(1))
                            .spacing(5),
                    ]
                        .spacing(15)
                        .width(Fill),
                    column![
                        text("Status").size(12),
                        job_status_select,
                    ]
                        .spacing(5),
                    row![
                        container(button(text("Save")).on_press(submit_message.clone()))
                        .width(Fill)
                        .align_x(Alignment::End),
                        button(text("Cancel")).on_press(Message::HideModal)
                    ]
                    .spacing(10)
                    .width(Fill),
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

    fn job_post_modal<'a>(&self, submit_message: Message) -> Element<'a, Message> {
        let title = match &self.job_post_id {
            Some(_) => "Edit Job Post",
            None => "New Job Post",
        };
        // let company name = 
        let company_element: Element<'_, Message, Theme, iced::Renderer> = match &self.job_post_id {
            Some(_) => {
                text(self.company_name.clone())
                    .into()
            },
            None => {
                text_input("", "")
                    .padding(5)
                    .into()
            }
        };
        let min_yoe = match self.min_yoe {
            Some(num) => num.to_string(),
            None => "".to_string(),
        };
        let max_yoe = match self.max_yoe {
            Some(num) => num.to_string(),
            None => "".to_string(),
        };
        let posted_btn: iced::widget::Button<'_, Message, Theme, iced::Renderer> = button(text("Pick")).on_press(Message::PickJobPosted);
        let job_posted_picker = date_picker(
            self.pick_job_posted,
            self.job_posted.unwrap_or(Date::today()),
            posted_btn,
            Message::CancelJobPostedPicker,
            Message::JobPostedChanged,
        );
        let posted = match &self.job_posted {
            Some(date) => format!("{}/{}/{}", date.month, date.day, date.year),
            None => "None".to_string(),
        };
        let loc_type_select: SelectionList<'_, JobPostLocationType, Message, Theme, iced::Renderer> = SelectionList::new_with(
            &JobPostLocationType::ALL,
            Message::LocationTypeChanged,
            12.0,
            5.0,
            style::selection_list::primary,
            self.location_type_index,
            Font::default(),
        )
            .height(Length::Fixed(70.0));
        container(
            column![
                text(title).size(24),
                column![
                    row![
                        // Company name
                        column![
                            text("Company").size(12),
                            company_element,
                        ]
                            .width(Length::FillPortion(1))
                            .spacing(5),
                        // Date posted
                        column![
                            text("Date Posted").size(12),
                            row![
                                text(posted),
                                job_posted_picker,
                            ]
                                .spacing(10)
                                .align_y(Alignment::Center),
                        ]
                            .width(Length::FillPortion(1))
                            .spacing(5),
                    ]
                        .spacing(15),
                    row![
                        // Title field
                        column![
                            text("Job Title").size(12),
                            text_input("", &self.job_title)
                            .on_input(Message::JobTitleChanged)
                            .on_submit(submit_message.clone())
                            .padding(5),
                            ]
                            .width(Length::FillPortion(1))
                            .spacing(5),
                        // URL
                        column![
                            text("Job URL").size(12),
                            text_input("", &self.url)
                                .on_input(Message::JobURLChanged)
                                .on_submit(submit_message.clone())
                                .padding(5)
                        ]
                            .width(Length::FillPortion(1))
                            .spacing(5),
                    ]
                        .spacing(15),
                    row![
                        // Location field
                        column![
                            text("Location").size(12),
                            text_input("", &self.location)
                                .on_input(Message::LocationChanged)
                                .on_submit(submit_message.clone())
                                .padding(5),
                        ]
                            .width(Length::FillPortion(1))
                            .spacing(5),
                        // Location type
                        column![
                            text("Location Type").size(12),
                            loc_type_select,
                                // .padding(5),
                        ]
                            .width(Length::FillPortion(1))
                            .spacing(5),
                    ]
                        .spacing(15),
                    row![
                        // Min years
                        column![
                            text("Min. Years").size(12),
                            text_input("", &min_yoe)
                                .on_input(Message::MinYOEChanged)
                                .on_submit(submit_message.clone())
                                .padding(5)
                        ]
                            .width(Length::FillPortion(1))
                            .spacing(5),
                        // Max years
                        column![
                            text("Max. Years").size(12),
                            text_input("", &max_yoe)
                                .on_input(Message::MaxYOEChanged)
                                .on_submit(submit_message.clone())
                                .padding(5)
                        ]
                            .width(Length::FillPortion(1))
                            .spacing(5),
                        // Min pay
                        column![
                            text("Min. Pay").size(12),
                            text_input("", &self.min_pay)
                                .on_input(Message::MinPayChanged)
                                .on_submit(submit_message.clone())
                                .padding(5)
                        ]
                            .width(Length::FillPortion(1))
                            .spacing(5),
                        // Max pay
                        column![
                            text("Max. Pay").size(12),
                            text_input("", &self.max_pay)
                                .on_input(Message::MaxPayChanged)
                                .on_submit(submit_message.clone())
                                .padding(5)
                        ]
                            .width(Length::FillPortion(1))
                            .spacing(5),
                    ]
                        .spacing(15),
                    row![
                        // Skills
                        column![
                                text("Skills").size(12),
                                text("Comma-separated").size(10),
                                text_input("", &self.skills)
                                    .on_input(Message::SkillsChanged)
                                    .on_submit(submit_message.clone())
                                    .padding(5)
                        ]
                            .width(Length::FillPortion(1))
                            .spacing(5),
                        // Benefits
                        column![
                            text("Benefits").size(12),
                            text("Comma-separated").size(10),
                            text_input("", &self.benefits)
                                .on_input(Message::BenefitsChanged)
                                .on_submit(submit_message.clone())
                                .padding(5)
                        ]
                            .width(Length::FillPortion(1))
                            .spacing(5),
                    ]
                        .spacing(15),
                    // Save row
                    row![
                        container(button(text("Save")).on_press(submit_message.clone()))
                            .width(Fill)
                            .align_x(Alignment::End),
                        button(text("Cancel")).on_press(Message::HideModal)
                    ]
                        .spacing(10)
                        .width(Fill)
                ]
                    .spacing(10)
            ]
                .spacing(5)
        )
            .width(500)
            .padding(10)
            .style(container::rounded_box)
            .into()
    }

    fn hide_modal(&mut self) {
        self.modal = Modal::None;
        self.company_name = "".to_string(); // hmm...
        self.careers_url = "".to_string();
        self.company_id = None;
        self.job_post_id = None;
        self.job_app_id = None;
        self.job_app_status = None;
        self.job_app_status_index = None;
        self.job_app_applied = None;
        self.pick_job_app_applied = false;
        self.job_app_responded = None;
        self.pick_job_app_responded = false;
        self.job_title = "".to_string();
        self.min_yoe = None;
        self.max_yoe = None;
        self.min_pay = "".to_string();
        self.max_pay = "".to_string();
        self.benefits = "".to_string();
        self.location = "".to_string();
        self.job_posted = None;
        self.pick_job_posted = false;
        self.location_type = None;
        self.location_type_index = None;
        self.skills = "".to_string();
        self.url = "".to_string();
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
            // Window
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
            // Company
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
            Message::ToggleCompanyDropdown(id) => {
                let current_val = match self.company_dropdowns.get(&id) {
                    Some(&status) => status,
                    None => false
                };
                self.company_dropdowns.insert(id, !current_val);
                Task::none()
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
            // Job Application
            Message::CreateApplication => {
                if self.job_app_status == None {
                    return Task::none() // TODO feedback
                }
                let new_app = JobApplication {
                    id: 0,
                    job_post_id: self.job_post_id.unwrap(),
                    status: self.job_app_status.clone().unwrap(),
                    date_applied: get_utc(self.job_app_applied),
                    date_responded: get_utc(self.job_app_responded),
                };
                let _ = JobApplication::create(&self.db, new_app);
                self.hide_modal();
                Task::none()
            }
            Message::EditApplication => {
                let app_id = match self.job_app_id {
                    Some(id) => id,
                    None => return Task::none(),
                };
                if self.job_app_status == None {
                    return Task::none() // TODO feedback
                }
                let app = JobApplication {
                    id: app_id,
                    job_post_id: self.job_post_id.unwrap(),
                    status: self.job_app_status.clone().unwrap(),
                    date_applied: get_utc(self.job_app_applied),
                    date_responded: get_utc(self.job_app_responded),
                };
                let _ = JobApplication::update(&self.db, app).expect("Failed to update application");
                self.hide_modal();
                Task::none()
            }
            // Job Post
            Message::DeleteJobPost(id) => {
                let _ = JobPost::delete(&self.db, id);
                self.job_posts = JobPost::get_all(&self.db).expect("Failed to get job posts");
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
            Message::EditJobPost => {
                let post_id = match self.job_post_id {
                    Some(id) => id,
                    None => return Task::none()
                };
                let mut post = self.job_posts.iter().find(|post| post.id == post_id).unwrap().clone();
                if self.location_type == None || self.location == "" || 
                    self.job_title == "" || self.url == "" {
                    return Task::none() // TODO feedback
                }
                let min_pay = match self.min_pay.as_str() {
                    "" => None,
                    _ => Some(get_pay_i64(&self.min_pay).unwrap()),
                };
                let max_pay = match self.max_pay.as_str() {
                    "" => None,
                    _ => Some(get_pay_i64(&self.max_pay).unwrap()),
                };
                post.location = self.location.clone();
                post.location_type = self.location_type.clone().unwrap();
                post.url = self.url.clone();
                post.min_yoe = self.min_yoe;
                post.max_yoe = self.max_yoe;
                post.min_pay_cents = min_pay;
                post.max_pay_cents = max_pay; 
                post.date_posted = get_utc(self.job_posted);
                post.job_title = self.job_title.clone();
                post.benefits = Some(self.benefits.clone());
                post.skills = Some(self.skills.clone());
                let _ = JobPost::update(&self.db, post).expect("Failed to update job post");
                self.job_posts = JobPost::get_all(&self.db).expect("Failed to get job posts");
                self.hide_modal();
                Task::none()
            }
            // Filter
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
            // Hide Modal
            Message::HideModal => {
                self.hide_modal();
                Task::none()
            }
            // Show modal
            Message::ShowCreateCompanyModal => {
                self.modal = Modal::CreateCompanyModal;
                focus_next()
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
            Message::ShowCreateApplicationModal(job_post_id) => {
                self.job_app_status_index = JobApplicationStatus::ALL.iter().position(|x| x == &JobApplicationStatus::New);
                self.job_app_status = Some(JobApplicationStatus::New);
                self.job_post_id = Some(job_post_id);
                // self.job_app_applied = Some(Date::today());
                self.modal = Modal::CreateApplicationModal;
                focus_next()
            }
            Message::ShowEditApplicationModal(application_id) => {
                let application = JobApplication::get(&self.db, application_id).unwrap();
                self.job_post_id = Some(application.job_post_id);
                self.job_app_id = Some(application.id);
                self.job_app_status_index = JobApplicationStatus::ALL.iter().position(|x| x == &application.status);
                self.job_app_status = Some(application.status);
                self.job_app_applied = get_iced_date(application.date_applied);
                self.job_app_responded = get_iced_date(application.date_responded);
                self.modal = Modal::EditApplicationModal;
                focus_next()
            }
            Message::ShowEditJobPostModal(job_post_id) => {
                let job_post = self.job_posts.iter().find(|post| post.id == job_post_id).unwrap();
                let company = self.companies.iter().find(|company| company.id == job_post.company_id).unwrap();
                self.company_name = company.name.clone();
                self.job_post_id = Some(job_post.id);
                self.company_id = Some(company.id);
                self.job_title = job_post.job_title.clone();
                self.job_posted = get_iced_date(job_post.date_posted);
                self.location = job_post.location.clone();
                self.location_type = Some(job_post.location_type.clone());
                self.location_type_index = JobPostLocationType::ALL.iter().position(|x| x == &job_post.location_type);
                self.min_yoe = job_post.min_yoe;
                self.max_yoe = job_post.max_yoe;
                self.min_pay = get_pay_str(job_post.min_pay_cents);
                self.max_pay = get_pay_str(job_post.max_pay_cents);
                self.benefits = job_post.benefits.clone().unwrap_or("".to_string());
                self.skills = job_post.skills.clone().unwrap_or("".to_string());
                self.url = job_post.url.clone();
                self.modal = Modal::EditJobPostModal;
                focus_next()
            }
            // Advanced modal fields
            Message::PickJobApplicationApplied => {
                self.pick_job_app_applied = true;
                Task::none()
            }
            Message::PickJobApplicationResponded => {
                self.pick_job_app_responded = true;
                Task::none()
            }
            Message::CancelJobApplicationPickers => {
                self.pick_job_app_applied = false;
                self.pick_job_app_responded = false;
                Task::none()
            }
            Message::PickJobPosted => {
                self.pick_job_posted = true;
                Task::none()
            }
            Message::CancelJobPostedPicker => {
                self.pick_job_posted = false;
                Task::none()
            }
            // Modal input
            Message::CompanyNameChanged(name) => {
                self.company_name = name; // hmm...
                Task::none()
            }
            Message::CareersURLChanged(careers_url) => {
                self.careers_url = careers_url;
                Task::none()
            }
            Message::JobApplicationStatusChanged(index, status) => {
                self.job_app_status = Some(status);
                self.job_app_status_index = Some(index);
                Task::none()
            }
            Message::JobApplicationAppliedChanged(date) => {
                self.job_app_applied = Some(date);
                self.pick_job_app_applied = false;
                Task::none()
            }
            Message::JobApplicationRespondedChanged(date) => {
                self.job_app_responded = Some(date);
                self.pick_job_app_responded = false;
                Task::none()
            }
            Message::JobTitleChanged(title) => {
                self.job_title = title;
                Task::none()
            }
            Message::MinYOEChanged(yoe_str) => {
                let yoe: Result<i32, _> = yoe_str.parse();
                match yoe {
                    Ok(num) => {
                        self.min_yoe = Some(num);
                    }
                    Err(_) => {
                        self.min_yoe = None;
                    }
                };
                Task::none()
            }
            Message::MaxYOEChanged(yoe_str)  => {
                let yoe: Result<i32, _> = yoe_str.parse();
                match yoe {
                    Ok(num) => {
                        self.max_yoe = Some(num);
                    }
                    Err(_) => {
                        self.max_yoe = None;
                    }
                };
                Task::none()
            }
            Message::MinPayChanged(pay_str) => {
                self.min_pay = pay_str;
                Task::none()
            }
            Message::MaxPayChanged(pay_str) => {
                self.max_pay = pay_str;
                Task::none()
            }
            Message::BenefitsChanged(benefits) => {
                self.benefits = benefits;
                Task::none()
            }
            Message::LocationChanged(location) => {
                self.location = location;
                Task::none()
            }
            Message::JobPostedChanged(date) => {
                self.job_posted = Some(date);
                self.pick_job_posted = false;
                Task::none()
            }
            Message::LocationTypeChanged(index, loc_type) => {
                self.location_type = Some(loc_type);
                self.location_type_index = Some(index);
                Task::none()
            }
            Message::JobURLChanged(url) => {
                self.url = url;
                Task::none()
            }
            Message::SkillsChanged(skills) => {
                self.skills = skills;
                Task::none()
            }
            // Event
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
                                                .on_press(Message::DeleteCompany(company_id)) // TODO warning / confirmation
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
                                    // let location_text = format!("{} ({})", &job_post.location, &job_post.location_type);
                                    let location_type_style = match &job_post.location_type {
                                        JobPostLocationType::Onsite => style::badge::warning,
                                        JobPostLocationType::Hybrid => style::badge::info,
                                        JobPostLocationType::Remote => style::badge::primary,
                                    };
                                    let posted_text = format!("{}", &job_post.date_posted.unwrap().format("%m/%d/%Y"));

                                    let min_yoe = &job_post.min_yoe.unwrap_or(-1);
                                    let max_yoe = &job_post.max_yoe.unwrap_or(-1);
                                    let yoe_text = match (*max_yoe > -1, *min_yoe > -1) {
                                        (true, true) => format!("{} - {} years", min_yoe, max_yoe),
                                        (false, true) => format!("{}+ years", min_yoe),
                                        _ => "No required years found".to_string(),
                                    };

                                    let min_pay = &job_post.min_pay_cents.unwrap_or(-1);
                                    let max_pay = &job_post.max_pay_cents.unwrap_or(-1);
                                    let pay_text = match (*max_pay > -1, *min_pay > -1) {
                                        (true, true) => format!("${} - ${}", get_pay_str(Some(*min_pay)), get_pay_str(Some(*max_pay))),
                                        (false, true) => format!("${}+", get_pay_str(Some(*min_pay))),
                                        (true, false) => format!("${}", get_pay_str(Some(*max_pay))),
                                        _ => "No salary specified".to_string(),
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
                                    let apply_text: &str;
                                    let apply_msg: Message;
                                    match app_id {
                                        Some(id) => {
                                            apply_text = "Apply";
                                            apply_msg = Message::ShowEditApplicationModal(id)
                                        },
                                        None => {
                                            apply_text = "Apply";
                                            apply_msg = Message::ShowCreateApplicationModal(job_post.id);
                                        },
                                    };
                                    let dropdown = DropDown::new(
                                        underlay,
                                        column(vec![
                                            button(text(apply_text))
                                                .on_press(apply_msg)
                                                .into(),
                                            button(text("Edit"))
                                                .on_press(Message::ShowEditJobPostModal(job_post.id))
                                                .into(),
                                            button(text("Delete")) // TODO warning/confirmation
                                                .on_press(Message::DeleteJobPost(job_post.id))
                                                .into(),
                                        ])
                                        .spacing(5),
                                        match self.job_dropdowns.get(&job_post.id) {
                                            Some(&status) => status,
                                            None => false,
                                        }
                                    )
                                    .width(Fill)
                                    .alignment(drop_down::Alignment::Bottom)
                                    .on_dismiss(Message::ToggleJobDropdown(job_post.id));

                                    let skills_text = match &job_post.skills {
                                        Some(skills) => format_comma_separated(skills.to_string()),
                                        None => "No skills specified".to_string(),
                                    };
                                    let benefits_text = match &job_post.benefits {
                                        Some(benefits) => format_comma_separated(benefits.to_string()),
                                        None => "No benefits specified".to_string(),
                                    };
                                    
                                    container(
                                        row![
                                            column![
                                                text(&job_post.job_title),
                                                text(company.name).size(12),
                                                row![
                                                    text(&job_post.location).size(12),
                                                    badge(text(format!("{}", &job_post.location_type)).size(12)).style(location_type_style),
                                                ]
                                                    .spacing(5)
                                                    .align_y(Alignment::Center),
                                                text(posted_text).size(12),
                                            ]
                                                .spacing(5)
                                                .width(Length::FillPortion(2)),
                                            column![
                                                text("Qualifications").size(12),
                                                text(yoe_text),
                                                text(skills_text),
                                            ]
                                                .spacing(5)
                                                .width(Length::FillPortion(2)),
                                            column![
                                                text("Compensation").size(12),
                                                text(pay_text),
                                                text(benefits_text),
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
            // Company Modals
            Modal::CreateCompanyModal => {
                let create_company_content = self.company_modal(Message::TrackNewCompany);

                modal(main_window_content, create_company_content, Message::HideModal)
            }
            Modal::EditCompanyModal => {
                let edit_company_content = self.company_modal(Message::EditCompany);

                modal(main_window_content, edit_company_content, Message::HideModal)
            }
            // Job Application Modals
            Modal::CreateApplicationModal => {
                let create_job_app_content = self.job_app_modal(Message::CreateApplication);

                modal(main_window_content, create_job_app_content, Message::HideModal)
            }
            Modal::EditApplicationModal => {
                let edit_job_app_content = self.job_app_modal(Message::EditApplication);

                modal(main_window_content, edit_job_app_content, Message::HideModal)
            }
            // Job Post Modals
            Modal::EditJobPostModal => {
                let edit_job_post_content = self.job_post_modal(Message::EditJobPost);

                modal(main_window_content, edit_job_post_content, Message::HideModal)
            }
            Modal::None | _ => {
                main_window_content.into()
            }
        }
    }
    
}
