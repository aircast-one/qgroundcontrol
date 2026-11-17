use tauri::menu::{IsMenuItem, Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub const MENU_EVENT: &str = "menu";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Command {
    pub id: &'static str,
    pub label: &'static str,
    pub accelerator: Option<&'static str>,
}

const fn command(
    id: &'static str,
    label: &'static str,
    accelerator: Option<&'static str>,
) -> Command {
    Command {
        id,
        label,
        accelerator,
    }
}

pub const ABOUT: Command = command("about", "About Aircast", None);
pub const SETTINGS: Command = command("settings", "Settings…", Some("CmdOrCtrl+,"));

pub const FILE: [Command; 4] = [
    command("new-plan", "New plan", Some("CmdOrCtrl+N")),
    command("open-plan", "Open plan…", Some("CmdOrCtrl+O")),
    command("save-plan", "Save plan", Some("CmdOrCtrl+S")),
    command("save-plan-as", "Save plan as…", Some("CmdOrCtrl+Shift+S")),
];

pub const VIEW: [Command; 4] = [
    command("fly", "Fly", Some("CmdOrCtrl+1")),
    command("plan", "Plan", Some("CmdOrCtrl+2")),
    command("analyze", "Analyze", Some("CmdOrCtrl+3")),
    command("setup", "Setup", Some("CmdOrCtrl+4")),
];

pub const FULL_SCREEN: Command = command("full-screen", "Full screen", Some("F11"));
pub const EXIT: Command = command("exit", "Exit", None);

pub const HELP: [(Command, &str); 4] = [
    (
        command("help-docs", "Aircast documentation", None),
        "https://aircast.one/docs",
    ),
    (
        command("help-troubleshooting", "Troubleshooting", None),
        "https://aircast.one/docs#troubleshooting",
    ),
    (
        command("help-releases", "Release notes", None),
        "https://aircast.one/docs/releases",
    ),
    (
        command("help-contact", "Contact support", None),
        "https://aircast.one/contact",
    ),
];

#[derive(Debug, PartialEq)]
pub enum Route {
    Page(&'static str),
    Link(&'static str),
    FullScreen,
    Exit,
}

pub fn route(id: &str) -> Option<Route> {
    let page = [ABOUT, SETTINGS]
        .iter()
        .chain(FILE.iter())
        .chain(VIEW.iter())
        .find(|command| command.id == id)
        .map(|command| Route::Page(command.id));
    let link = HELP
        .iter()
        .find(|(command, _)| command.id == id)
        .map(|(_, url)| Route::Link(url));
    let window = match id {
        _ if id == FULL_SCREEN.id => Some(Route::FullScreen),
        _ if id == EXIT.id => Some(Route::Exit),
        _ => None,
    };
    page.or(link).or(window)
}

fn item<R: Runtime>(
    app: &AppHandle<R>,
    command: Command,
) -> tauri::Result<tauri::menu::MenuItem<R>> {
    command
        .accelerator
        .into_iter()
        .fold(
            MenuItemBuilder::with_id(command.id, command.label),
            |built, accelerator| built.accelerator(accelerator),
        )
        .build(app)
}

fn items<R: Runtime>(
    app: &AppHandle<R>,
    commands: &[Command],
) -> tauri::Result<Vec<tauri::menu::MenuItem<R>>> {
    commands.iter().map(|command| item(app, *command)).collect()
}

fn refs<R: Runtime>(items: &[tauri::menu::MenuItem<R>]) -> Vec<&dyn IsMenuItem<R>> {
    items.iter().map(|it| it as &dyn IsMenuItem<R>).collect()
}

#[cfg(target_os = "macos")]
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let about = item(app, ABOUT)?;
    let settings = item(app, SETTINGS)?;
    let files = items(app, &FILE)?;
    let views = items(app, &VIEW)?;
    let help = items(
        app,
        &HELP.iter().map(|(command, _)| *command).collect::<Vec<_>>(),
    )?;
    let app_menu = SubmenuBuilder::new(app, "Aircast")
        .item(&about)
        .separator()
        .item(&settings)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;
    let file = SubmenuBuilder::new(app, "File")
        .items(&refs(&files[..2]))
        .separator()
        .items(&refs(&files[2..]))
        .separator()
        .close_window()
        .build()?;
    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    let view = SubmenuBuilder::new(app, "View")
        .items(&refs(&views))
        .separator()
        .fullscreen()
        .build()?;
    let window = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .separator()
        .bring_all_to_front()
        .build()?;
    window.set_as_windows_menu_for_nsapp()?;
    let help_menu = SubmenuBuilder::new(app, "Help")
        .items(&refs(&help))
        .build()?;
    help_menu.set_as_help_menu_for_nsapp()?;
    MenuBuilder::new(app)
        .items(&[&app_menu, &file, &edit, &view, &window, &help_menu])
        .build()
}

#[cfg(not(target_os = "macos"))]
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let settings = item(app, SETTINGS)?;
    let exit = item(app, EXIT)?;
    let about = item(app, ABOUT)?;
    let full_screen = item(app, FULL_SCREEN)?;
    let files = items(app, &FILE)?;
    let views = items(app, &VIEW)?;
    let help = items(
        app,
        &HELP.iter().map(|(command, _)| *command).collect::<Vec<_>>(),
    )?;
    let file = SubmenuBuilder::new(app, "File")
        .items(&refs(&files[..2]))
        .separator()
        .items(&refs(&files[2..]))
        .separator()
        .item(&settings)
        .separator()
        .item(&exit)
        .build()?;
    let edit = SubmenuBuilder::new(app, "Edit")
        .cut()
        .copy()
        .paste()
        .separator()
        .select_all()
        .build()?;
    let view = SubmenuBuilder::new(app, "View")
        .items(&refs(&views))
        .separator()
        .item(&full_screen)
        .build()?;
    let window = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .build()?;
    let help_menu = SubmenuBuilder::new(app, "Help")
        .items(&refs(&help))
        .separator()
        .item(&about)
        .build()?;
    MenuBuilder::new(app)
        .items(&[&file, &edit, &view, &window, &help_menu])
        .build()
}

pub fn handle<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match route(id) {
        Some(Route::Page(command)) => {
            if let Err(error) = app.emit(MENU_EVENT, command) {
                log::warn!(target: "desktop.menu", "event=menu_not_delivered id={command} reason=\"{error}\"");
            }
        }
        Some(Route::Link(url)) => match url::Url::parse(url) {
            Ok(url) => crate::open_link(&url),
            Err(error) => {
                log::warn!(target: "desktop.menu", "event=link_invalid reason=\"{error}\"")
            }
        },
        Some(Route::FullScreen) => {
            if let Some(window) = app.get_webview_window("main")
                && let Err(error) = crate::window::toggle_full_screen(window)
            {
                log::warn!(target: "desktop.menu", "event=full_screen_failed reason=\"{error}\"");
            }
        }
        Some(Route::Exit) => {
            if let Some(window) = app.get_webview_window("main")
                && let Err(error) = window.close()
            {
                log::warn!(target: "desktop.menu", "event=exit_failed reason=\"{error}\"");
            }
        }
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accelerators() -> Vec<&'static str> {
        [ABOUT, SETTINGS, FULL_SCREEN, EXIT]
            .iter()
            .chain(FILE.iter())
            .chain(VIEW.iter())
            .chain(HELP.iter().map(|(command, _)| command))
            .filter_map(|command| command.accelerator)
            .collect()
    }

    #[test]
    fn every_command_routes_to_the_page_a_link_or_the_window() {
        assert_eq!(route("settings"), Some(Route::Page("settings")));
        assert_eq!(route("about"), Some(Route::Page("about")));
        assert_eq!(route("save-plan-as"), Some(Route::Page("save-plan-as")));
        assert_eq!(route("setup"), Some(Route::Page("setup")));
        assert_eq!(route("full-screen"), Some(Route::FullScreen));
        assert_eq!(route("exit"), Some(Route::Exit));
        assert_eq!(
            route("quit"),
            None,
            "Quit is the predefined item, held by applicationShouldTerminate"
        );
        let pages = [ABOUT, SETTINGS]
            .iter()
            .chain(FILE.iter())
            .chain(VIEW.iter());
        pages.for_each(|command| assert_eq!(route(command.id), Some(Route::Page(command.id))));
    }

    #[test]
    fn help_opens_aircast_pages_not_qgroundcontrol() {
        HELP.iter().for_each(|(command, url)| {
            assert_eq!(route(command.id), Some(Route::Link(url)));
            let parsed = url::Url::parse(url).unwrap();
            assert_eq!(parsed.scheme(), "https");
            assert_eq!(parsed.host_str(), Some("aircast.one"));
        });
        assert_eq!(
            route("help-troubleshooting"),
            Some(Route::Link("https://aircast.one/docs#troubleshooting")),
            "aircast.one has no troubleshooting index page; the docs list its guides under this heading"
        );
    }

    #[test]
    fn full_screen_reads_right_whether_the_window_is_in_it_or_not() {
        assert_eq!(FULL_SCREEN.label, "Full screen");
    }

    #[test]
    fn no_key_is_bound_twice_and_ids_are_unique() {
        let keys = accelerators();
        let unique: std::collections::HashSet<_> = keys.iter().collect();
        assert_eq!(unique.len(), keys.len(), "{keys:?}");
        let ids: Vec<_> = [ABOUT, SETTINGS, FULL_SCREEN, EXIT]
            .iter()
            .chain(FILE.iter())
            .chain(VIEW.iter())
            .chain(HELP.iter().map(|(command, _)| command))
            .map(|command| command.id)
            .collect();
        let distinct: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(distinct.len(), ids.len(), "{ids:?}");
        assert!(
            keys.contains(&"CmdOrCtrl+,"),
            "Settings… is Cmd+, as on every Mac app"
        );
    }
}
