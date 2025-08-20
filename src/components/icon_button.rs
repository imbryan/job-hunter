use iced::{
    color,
    widget::{button, row, text},
    Alignment, Element,
};
use iced_font_awesome::{fa_icon, fa_icon_solid};

#[derive(Debug, Clone)]
pub enum IconButtonMessage {
    Pressed,
}

pub struct IconButton<'a> {
    pub icon_name: &'a str,
    pub label: Option<&'a str>,
    pub solid: bool,
}

impl<'a> IconButton<'a> {
    pub fn new(icon_name: &'a str) -> Self {
        Self {
            icon_name,
            label: None,
            solid: false,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn solid(mut self) -> Self {
        self.solid = true;
        self
    }

    pub fn view(self) -> Element<'a, IconButtonMessage> {
        let mut content = row![];
        if let Some(label) = self.label {
            content = content.push(text(label));
        }

        let icon = if self.solid {
            fa_icon_solid(self.icon_name)
        } else {
            fa_icon(self.icon_name)
        };

        content = content.push(icon.color(color!(255, 255, 255)).size(15.0));

        button(content.spacing(5).align_y(Alignment::Center))
            .on_press(IconButtonMessage::Pressed)
            .into()
    }
}
