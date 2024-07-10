use iced::Border;
use iced::widget::button;
use iced::widget::button::Appearance;

use crate::theme::{BUTTON_BORDER_RADIUS, DANGER, GauntletSettingsTheme, PRIMARY, PRIMARY_HOVERED, SUCCESS, TEXT, TEXT_DARK};

#[derive(Default)]
pub enum ButtonStyle {
    #[default]
    Primary,
    Positive,
    Destructive,
    TableRow,
}

//noinspection RsSortImplTraitMembers
impl button::StyleSheet for GauntletSettingsTheme {
    type Style = ButtonStyle;

    fn active(&self, style: &Self::Style) -> Appearance {
        let (background_color, text_color) = match style {
            ButtonStyle::Primary => (PRIMARY.to_iced(), TEXT_DARK.to_iced()),
            ButtonStyle::Positive => (SUCCESS.to_iced(), TEXT_DARK.to_iced()),
            ButtonStyle::Destructive => (DANGER.to_iced(), TEXT.to_iced()),
            ButtonStyle::TableRow => {
                return Appearance {
                    background: None,
                    text_color: TEXT.to_iced(),
                    ..Default::default()
                }
            }
        };

        Appearance {
            background: Some(background_color.into()),
            text_color,
            border: Border {
                radius: BUTTON_BORDER_RADIUS.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> Appearance {
        let (background_color, text_color) = match style {
            ButtonStyle::Primary => (PRIMARY_HOVERED.to_iced(), TEXT_DARK.to_iced()),
            ButtonStyle::Positive => (SUCCESS.to_iced(), TEXT_DARK.to_iced()), // TODO
            ButtonStyle::Destructive => (DANGER.to_iced(), TEXT.to_iced()), // TODO
            ButtonStyle::TableRow => {
                return Appearance {
                    background: None,
                    text_color: TEXT.to_iced(), // TODO
                    ..Default::default()
                }
            }
        };

        Appearance {
            background: Some(background_color.into()),
            text_color,
            border: Border {
                radius: BUTTON_BORDER_RADIUS.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn focused(&self, style: &Self::Style, _is_active: bool) -> Appearance {
        self.hovered(style)
    }
}