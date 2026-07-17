use std::cell::Cell;

use termusiclib::config::SharedTuiSettings;
use termusiclib::player::{SortCriterion, SortDirection};
use tui_realm_stdlib::components::Table;
use tui_realm_stdlib::prop_ext::CommonHighlight;
use tuirealm::command::{Cmd, Direction};
use tuirealm::component::{AppComponent, Component};
use tuirealm::event::{Event, Key, KeyEvent, KeyModifiers};
use tuirealm::props::{
    AttrValue, BorderType, Borders, HorizontalAlignment, LineStatic, PropPayload, PropValue, Style,
    TableBuilder, Title,
};
use tuirealm::state::{State, StateValue};

use crate::ui::ids::Id;
use crate::ui::model::{Model, UserEvent};
use crate::ui::msg::{Msg, SortPopupMsg};

struct SortHint {
    ascending_key: char,
    descending_key: char,
    criterion: SortCriterion,
    label: &'static str,
    asc_description: &'static str,
    desc_description: &'static str,
}

const SORT_HINTS: &[SortHint] = &[
    SortHint {
        ascending_key: 'a',
        descending_key: 'A',
        criterion: SortCriterion::Alphanumeric,
        label: "Alphanumeric",
        asc_description: "Filename A\u{2192}Z",
        desc_description: "Filename Z\u{2192}A",
    },
    SortHint {
        ascending_key: 't',
        descending_key: 'T',
        criterion: SortCriterion::FirstAdded,
        label: "First Added",
        asc_description: "Date added oldest\u{2192}newest",
        desc_description: "Date added newest\u{2192}oldest",
    },
    SortHint {
        ascending_key: 'd',
        descending_key: 'D',
        criterion: SortCriterion::Duration,
        label: "Duration",
        asc_description: "Length shortest\u{2192}longest",
        desc_description: "Length longest\u{2192}shortest",
    },
];

const TITLE_ASC: &str = " Sort (Ascending) \u{2014} Tab: toggle, Enter: select, q/Esc: cancel ";
const TITLE_DESC: &str = " Sort (Descending) \u{2014} Tab: toggle, Enter: select, q/Esc: cancel ";

/// Build the table content (rows) for the given sort direction.
fn table_data(direction: SortDirection) -> tuirealm::props::Table {
    let mut builder = TableBuilder::default();
    for (idx, hint) in SORT_HINTS.iter().enumerate() {
        let desc = match direction {
            SortDirection::Asc => hint.asc_description,
            SortDirection::Desc => hint.desc_description,
        };
        builder
            .add_col(LineStatic::from(format!(
                "  {} / {}",
                hint.ascending_key, hint.descending_key
            )))
            .add_col(LineStatic::from(hint.label))
            .add_col(LineStatic::from(desc));
        if idx < SORT_HINTS.len() - 1 {
            builder.add_row();
        }
    }
    builder.build()
}

fn build_table(config: &SharedTuiSettings, direction: SortDirection) -> Table {
    let config = config.read();
    let table = table_data(direction);
    let title = match direction {
        SortDirection::Asc => TITLE_ASC,
        SortDirection::Desc => TITLE_DESC,
    };

    Table::default()
        .borders(
            Borders::default()
                .modifiers(BorderType::Rounded)
                .color(config.settings.theme.fallback_border()),
        )
        .inactive(Style::new().bg(config.settings.theme.library_background()))
        .foreground(config.settings.theme.fallback_foreground())
        .background(config.settings.theme.fallback_background())
        .highlight_style(
            CommonHighlight::default()
                .style
                .fg(config.settings.theme.fallback_highlight()),
        )
        .scroll(true)
        .title(Title::from(title).alignment(HorizontalAlignment::Center))
        .rewind(false)
        .step(1)
        .row_height(1)
        .headers(["  Key", "Name", "Description"])
        .column_spacing(3)
        .widths(&[12, 20, 46])
        .table(table)
}

#[derive(Component)]
pub struct SortPopup {
    component: Table,
    direction: Cell<SortDirection>,
}

impl SortPopup {
    pub fn new(config: &SharedTuiSettings) -> Self {
        let component = build_table(config, SortDirection::Asc);
        Self {
            component,
            direction: Cell::new(SortDirection::Asc),
        }
    }

    fn match_key(ch: char) -> Option<(SortCriterion, SortDirection)> {
        for hint in SORT_HINTS {
            if ch == hint.ascending_key {
                return Some((hint.criterion, SortDirection::Asc));
            }
            if ch == hint.descending_key {
                return Some((hint.criterion, SortDirection::Desc));
            }
        }
        None
    }

    fn rebuild(&mut self, direction: SortDirection) {
        // Update the existing component in place so it keeps its `is_active`
        // flag (and therefore its visible row highlight). Replacing the whole
        // component would drop the active state, making the highlight disappear.
        let idx = match self.component.state() {
            State::Single(StateValue::Usize(i)) => Some(i),
            _ => None,
        };
        let title = match direction {
            SortDirection::Asc => TITLE_ASC,
            SortDirection::Desc => TITLE_DESC,
        };
        self.component.attr(
            tuirealm::props::Attribute::Content,
            AttrValue::Table(table_data(direction)),
        );
        self.component.attr(
            tuirealm::props::Attribute::Title,
            AttrValue::Title(Title::from(title).alignment(HorizontalAlignment::Center)),
        );
        if let Some(i) = idx {
            self.component.attr(
                tuirealm::props::Attribute::Value,
                AttrValue::Payload(PropPayload::Single(PropValue::Usize(i))),
            );
        }
    }

    fn selected_criterion(&self) -> Option<(SortCriterion, SortDirection)> {
        let State::Single(StateValue::Usize(idx)) = self.component.state() else {
            return None;
        };
        SORT_HINTS
            .get(idx)
            .map(|h| (h.criterion, self.direction.get()))
    }
}

impl AppComponent<Msg, UserEvent> for SortPopup {
    fn on(&mut self, ev: &Event<UserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char(ch),
                ..
            }) => {
                if *ch == 'q' {
                    return Some(Msg::SortPopup(SortPopupMsg::Close));
                }
                if let Some((criterion, direction)) = Self::match_key(*ch) {
                    return Some(Msg::SortPopup(SortPopupMsg::Selected(criterion, direction)));
                }
                None
            }
            Event::Keyboard(KeyEvent {
                code: Key::Esc,
                modifiers: KeyModifiers::NONE,
            }) => Some(Msg::SortPopup(SortPopupMsg::Close)),
            Event::Keyboard(KeyEvent {
                code: Key::Enter,
                modifiers: KeyModifiers::NONE,
            }) => self
                .selected_criterion()
                .map(|(c, d)| Msg::SortPopup(SortPopupMsg::Selected(c, d))),
            Event::Keyboard(KeyEvent {
                code: Key::Tab,
                modifiers: KeyModifiers::NONE,
            }) => {
                let new_dir = match self.direction.get() {
                    SortDirection::Asc => SortDirection::Desc,
                    SortDirection::Desc => SortDirection::Asc,
                };
                self.direction.set(new_dir);
                self.rebuild(new_dir);
                None
            }
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => {
                self.perform(Cmd::Move(Direction::Up));
                None
            }
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            }) => {
                self.perform(Cmd::Move(Direction::Down));
                None
            }
            _ => None,
        }
    }
}

impl Model {
    pub fn mount_sort_popup(&mut self) {
        assert!(
            self.app
                .remount(
                    Id::SortPopup,
                    Box::new(SortPopup::new(&self.config_tui)),
                    vec![]
                )
                .is_ok()
        );
        self.update_photo().ok();
        assert!(self.app.active(&Id::SortPopup).is_ok());
    }

    pub fn umount_sort_popup(&mut self) {
        self.app.umount(&Id::SortPopup).ok();
    }
}
