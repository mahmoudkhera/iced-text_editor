use iced::highlighter;
use iced::widget::{
    self, button, column, container, horizontal_space, pick_list, row, text, text_editor, tooltip,
};
use iced::Theme;
use iced::{Element, Font, Settings, Task};

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

struct Editor {
    content: widget::text_editor::Content,
    path: Option<PathBuf>,
    error: Option<Error>,
    theme: highlighter::Theme,
    is_dirty: bool,
}

#[derive(Debug, Clone)]
enum Message {
    Edit(widget::text_editor::Action),
    New,
    Open,
    FileOpen(Result<(PathBuf, Arc<String>), Error>),
    Save,
    FileSaved(Result<PathBuf, Error>),
    ThemeSelected(highlighter::Theme),
}
impl Editor {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                content: text_editor::Content::new(),
                path: None,
                error: None,
                theme: highlighter::Theme::SolarizedDark,
                is_dirty: true,
            },
            Task::perform(load_file(default_file()), Message::FileOpen),
        )
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Edit(action) => {
                self.is_dirty = self.is_dirty || action.is_edit();
                self.content.perform(action);

                Task::none()
            }
            Message::New => {
                self.path = None;
                self.content = text_editor::Content::new();
                self.error = None;
                Task::none()
            }
            Message::Open => Task::perform(pick_afile(), Message::FileOpen),
            Message::Save => {
                let text = self.content.text();

                Task::perform(file_saved(self.path.clone(), text), Message::FileSaved)
            }
            Message::FileSaved(Ok(path)) => {
                self.path = Some(path);
                self.is_dirty = false;
                Task::none()
            }
            Message::FileSaved(Err(error)) => {
                self.error = Some(error);
                Task::none()
            }

            Message::FileOpen(Ok((path, content))) => {
                self.path = Some(path);
                self.is_dirty = false;
                self.content = text_editor::Content::with_text(&content);
                Task::none()
            }
            Message::FileOpen(Err(error)) => {
                self.error = Some(error);
                Task::none()
            }

            Message::ThemeSelected(theme) => {
                self.theme = theme;
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let controls = widget::row![
            action(new_icon(), Some(Message::New), "new file"),
            action(open_icon(), Some(Message::Open), "open file"),
            action(
                save_icon(),
                self.is_dirty.then_some(Message::Save),
                "save file"
            ),
            horizontal_space(),
            pick_list(
                highlighter::Theme::ALL,
                Some(self.theme),
                Message::ThemeSelected
            )
        ]
        .spacing(10);
        let input = widget::text_editor(&self.content)
            .highlight(
                self.path
                    .as_ref()
                    .and_then(|path| path.extension()?.to_str())
                    .unwrap_or("rs"),
                self.theme,
            )
            .height(iced::Length::Fill)
            .on_action(Message::Edit);

        let status_bar = {
            let status = if let Some(Error::Io(error)) = self.error {
                text(error.to_string())
            } else {
                match self.path.as_deref().and_then(Path::to_str) {
                    Some(path) => text(path).size(14),
                    None => text("new file").size(14),
                }
            };

            let postion = {
                let (line, column) = self.content.cursor_position();
                text(format!("{}:{}", line + 1, column + 1))
            };

            row![status, horizontal_space(), postion]
        };

        widget::container(column![controls, input, status_bar])
            .padding(10)
            .into()
    }

    fn theme(&self) -> Theme {
        if self.theme.is_dark() {
            Theme::Dark
        } else {
            Theme::Light
        }
    }
}

async fn pick_afile() -> Result<(PathBuf, Arc<String>), Error> {
    let handle = rfd::AsyncFileDialog::new()
        .set_title("choose a file...")
        .pick_file()
        .await
        .ok_or(Error::FileDialogClosed)?;

    load_file(handle.path().to_owned()).await
}

async fn load_file(path: PathBuf) -> Result<(PathBuf, Arc<String>), Error> {
    let content = tokio::fs::read_to_string(&path)
        .await
        .map(Arc::new)
        .map_err(|error| error.kind())
        .map_err(Error::Io)?;
    Ok((path, content))
}
async fn file_saved(path: Option<PathBuf>, text: String) -> Result<PathBuf, Error> {
    let path = if let Some(path) = path {
        path
    } else {
        rfd::AsyncFileDialog::new()
            .set_title("choose file name")
            .save_file()
            .await
            .ok_or(Error::FileDialogClosed)
            .map(|file_handle| file_handle.path().to_owned())?
    };
    tokio::fs::write(&path, text)
        .await
        .map_err(|err| Error::Io(err.kind()))?;
    Ok(path)
}
fn default_file() -> PathBuf {
    PathBuf::from(format!("{}\\src\\main.rs", env!("CARGO_MANIFEST_DIR")))
}
fn icon(codepoint: char) -> Element<'static, Message> {
    const ICON_FONT: Font = iced::Font::with_name("editor-icons");

    text(codepoint).font(ICON_FONT).into()
}
fn new_icon() -> Element<'static, Message> {
    icon('\u{E800}')
}
fn open_icon() -> Element<'static, Message> {
    icon('\u{F15C}')
}
fn save_icon() -> Element<'static, Message> {
    icon('\u{E801}')
}
fn action<'a>(
    content: Element<'a, Message>,
    on_press: Option<Message>,
    label: &'a str,
) -> Element<'a, Message> {
    tooltip(
        button(container(content).width(30).center_x(30))
            .on_press_maybe(on_press)
            .padding([5, 10])
            .style(|theme: &Theme, status| match status {
                button::Status::Pressed => button::secondary(theme, status),

                _ => button::primary(theme, status),
            }),
        widget::Text::new(label).size(18),
        tooltip::Position::Bottom,
    )
    .style(|them| {
        let plate = them.palette();
        plate.background.into()
    })
    .into()
}

#[derive(Debug, Clone)]
enum Error {
    FileDialogClosed,
    Io(io::ErrorKind),
}

fn main() -> iced::Result {
    iced::application("TextEditor", Editor::update, Editor::view)
        .settings(Settings {
            fonts: vec![include_bytes!("../fonts/editor-icons.ttf")
                .as_slice()
                .into()],
            default_font: Font::MONOSPACE,
            ..Default::default()
        })
        .theme(Editor::theme)
        .run_with(|| Editor::new())
}
