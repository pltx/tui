//! Configuration should not be more than three levels deep, e.g.,
//! `config.one.two.three`.

use std::str::FromStr;

use color_eyre::Result;
use pltx_utils::dirs;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

const COLOR_PRESETS: [&str; 1] = ["default"];

/// The base/merged colors config.
#[derive(Deserialize, Serialize, Clone)]
pub struct ColorsConfig<S = String, C = Color> {
    // TODO: color presets to be implemented
    pub preset: S,
    pub fg: C,
    pub bg: C,
    pub secondary_fg: C,
    pub tertiary_fg: C,
    pub highlight_fg: C,
    pub primary: C,
    pub success: C,
    pub warning: C,
    pub danger: C,
    pub date_fg: C,
    pub time_fg: C,
    pub input_fg: C,
    pub input_bg: C,
    pub input_focus_fg: C,
    pub input_focus_bg: C,
    pub input_cursor_fg: C,
    pub input_cursor_bg: C,
    pub input_cursor_insert_fg: C,
    pub input_cursor_insert_bg: C,
    pub active_fg: C,
    pub active_bg: C,
    pub border: C,
    pub border_active: C,
    pub border_insert: C,
    pub popup_bg: C,
    pub popup_border: C,
    pub keybind_key: C,
    pub keybind_fg: C,
    pub title_bar_bg: C,
    pub title_bar_fg: C,
    pub tab_fg: C,
    pub tab_active_fg: C,
    pub tab_border: C,
    pub status_bar_bg: C,
    pub status_bar_fg: C,
    pub status_bar_normal_mode_bg: C,
    pub status_bar_normal_mode_fg: C,
    pub status_bar_insert_mode_bg: C,
    pub status_bar_insert_mode_fg: C,
    pub status_bar_interactive_mode_bg: C,
    pub status_bar_interactive_mode_fg: C,
    pub status_bar_delete_mode_bg: C,
    pub status_bar_delete_mode_fg: C,
}

/// The base/merged home module config.
#[derive(Deserialize, Serialize, Clone)]
pub struct HomeModule<S> {
    pub dashboard_title: S,
    pub dashboard_message: S,
}

/// The base/merged project management config.
#[derive(Deserialize, Serialize, Clone)]
pub struct ProjectMangementModule<N, C> {
    pub max_lists: N,
    pub due_soon_days: N,
    pub completed_char: C,
    pub overdue_char: C,
    pub due_soon_char: C,
    pub in_progress_char: C,
    pub important_char: C,
    pub default_char: C,
}

/// The base/merged modules config.
#[derive(Clone)]
pub struct ModulesConfig {
    pub home: HomeModule<String>,
    pub project_management: ProjectMangementModule<i32, String>,
}

/// The user modules config.
#[derive(Deserialize, Serialize)]
pub struct ModulesConfigFile {
    pub home: Option<HomeModule<Option<String>>>,
    pub project_management: Option<ProjectMangementModule<Option<i32>, Option<String>>>,
}

/// The base/merged profile config
#[derive(Clone, Deserialize, Serialize)]
pub struct ProfileConfig<S = String> {
    pub name: S,
    pub config_file: S,
    pub db_file: S,
    pub log_file: S,
}

/// The user config.
#[derive(Deserialize, Serialize)]
struct ConfigFile {
    log_level: Option<String>,
    default_profile: Option<String>,
    profiles: Option<Vec<ProfileConfig<Option<String>>>>,
    colors: Option<ColorsConfig<Option<String>, Option<String>>>,
    modules: Option<ModulesConfigFile>,
}

/// The main base/merged config.
#[derive(Clone)]
pub struct Config {
    pub log_level: String,
    pub default_profile: Option<String>,
    pub colors: ColorsConfig,
    pub modules: ModulesConfig,
    pub profiles: Vec<ProfileConfig<String>>,
}

/// Default config values. Overridden if user config values are provided.
fn base_config() -> Config {
    // NOTE: Remember to update `README.md` with the default configuration values.
    Config {
        log_level: String::from("info"),
        default_profile: None,
        colors: {
            let fg = "#c0caf5";
            let secondary_fg = "#7f87ac";
            let tertiary_fg = "#2c344d";
            let bg = "#11121D";
            let secondary_bg = "#232b44";
            let tertiary_bg = "#373f58";

            ColorsConfig {
                preset: String::from("default"),
                fg: get_color(fg),
                secondary_fg: get_color(secondary_fg),
                tertiary_fg: get_color(tertiary_fg),
                highlight_fg: get_color("#61a4ff"),
                bg: get_color(bg),
                primary: get_color("#9556f7"),
                success: get_color("#85f67a"),
                warning: get_color("#ff9382"),
                danger: get_color("#ff4d66"),
                date_fg: get_color("#9293b8"),
                time_fg: get_color("#717299"),
                input_fg: get_color(fg),
                input_bg: get_color(secondary_bg),
                input_focus_fg: get_color(fg),
                input_focus_bg: get_color(tertiary_bg),
                input_cursor_fg: get_color("#000000"),
                input_cursor_bg: get_color(secondary_fg),
                input_cursor_insert_fg: get_color("#000000"),
                input_cursor_insert_bg: get_color(fg),
                active_fg: get_color(secondary_bg),
                active_bg: get_color("#00ffff"),
                border: get_color(secondary_bg),
                border_active: get_color("#4d556e"),
                border_insert: get_color("#00FFFF"),
                popup_bg: get_color(bg),
                popup_border: get_color("#A485DD"),
                keybind_key: get_color("#A485DD"),
                keybind_fg: get_color("#6698FF"),
                title_bar_bg: get_color(tertiary_bg),
                title_bar_fg: get_color("#CCCCCC"),
                tab_fg: get_color(secondary_fg),
                tab_active_fg: get_color(fg),
                tab_border: get_color(tertiary_bg),
                status_bar_bg: get_color(secondary_bg),
                status_bar_fg: get_color(secondary_fg),
                status_bar_normal_mode_bg: get_color("#9bff46"),
                status_bar_normal_mode_fg: get_color(secondary_bg),
                status_bar_insert_mode_bg: get_color("#00ffff"),
                status_bar_insert_mode_fg: get_color(secondary_bg),
                status_bar_interactive_mode_bg: get_color("#ffff32"),
                status_bar_interactive_mode_fg: get_color(secondary_bg),
                status_bar_delete_mode_bg: get_color("#ff4d66"),
                status_bar_delete_mode_fg: get_color(secondary_bg),
            }
        },
        modules: ModulesConfig {
            home: HomeModule {
                dashboard_title: String::from("Privacy Life Tracker X"),
                dashboard_message: String::from(
                    "Manage your personal life privately and securely.",
                ),
            },
            project_management: ProjectMangementModule {
                max_lists: 5,
                due_soon_days: 3,
                completed_char: String::from("✅"),
                overdue_char: String::from("🚫"),
                due_soon_char: String::from("⏰"),
                in_progress_char: String::from("🌐"),
                important_char: String::from("⭐"),
                default_char: String::from(" "),
            },
        },
        profiles: vec![ProfileConfig {
            name: String::from("dev"),
            config_file: String::from("dev.toml"),
            db_file: String::from("dev.db"),
            log_file: String::from("dev.log"),
        }],
    }
}

fn base_dev_config(base_config: Config) -> Config {
    let mut dev_config = base_config;
    dev_config.log_level = String::from("debug");
    dev_config.modules.home.dashboard_title = String::from("DEVELOPER PROFILE ENABLED");
    dev_config.modules.home.dashboard_message = String::from("This profile's data is separate!");
    dev_config
}

/// Read the config file if it exists.
fn read_config_file(filename: &str) -> Result<Option<ConfigFile>> {
    let config_file = dirs::config_dir().join(filename);
    let config_contents: Option<String> = std::fs::read_to_string(config_file).ok();
    let config_toml: Option<ConfigFile> = match config_contents {
        Some(contents) => toml::from_str(&contents).expect("the config is invalid"),
        None => None,
    };
    Ok(config_toml)
}

/// Get the ratatui compatible color from a hex color.
fn get_color(color: &str) -> Color {
    Color::from_str(color).expect("failed to get color from string")
}

/// Call the `get_color()` function if provided (from user config), otherwise
/// return the base config value.
fn color_op(color_op: Option<String>, base_config_color: Color) -> Color {
    match color_op {
        Some(color) => get_color(&color),
        None => base_config_color,
    }
}

// TODO: Optimisation. There is lots of clones to reduce the level of nesting.
// Try to not nest too deeply to keep the code easier to read and maintain.
/// Merge the user config with the base config.
fn merge_config(user_config: ConfigFile, base_config: Config) -> Config {
    let colors = user_config.colors.map(|a| {
        let b = base_config.colors.clone();
        ColorsConfig {
            preset: a
                .preset
                .filter(|p| COLOR_PRESETS.iter().any(|&cp| cp == p))
                .unwrap_or(b.preset),
            fg: color_op(a.fg, b.fg),
            secondary_fg: color_op(a.secondary_fg, b.secondary_fg),
            tertiary_fg: color_op(a.tertiary_fg, b.tertiary_fg),
            highlight_fg: color_op(a.highlight_fg, b.highlight_fg),
            bg: color_op(a.bg, b.bg),
            primary: color_op(a.primary, b.primary),
            success: color_op(a.success, b.success),
            warning: color_op(a.warning, b.warning),
            danger: color_op(a.danger, b.danger),
            date_fg: color_op(a.date_fg, b.date_fg),
            time_fg: color_op(a.time_fg, b.time_fg),
            input_fg: color_op(a.input_fg, b.input_fg),
            input_bg: color_op(a.input_bg, b.input_bg),
            input_focus_fg: color_op(a.input_focus_fg, b.input_focus_fg),
            input_focus_bg: color_op(a.input_focus_bg, b.input_focus_bg),
            input_cursor_fg: color_op(a.input_cursor_fg, b.input_cursor_fg),
            input_cursor_bg: color_op(a.input_cursor_bg, b.input_cursor_bg),
            input_cursor_insert_fg: color_op(a.input_cursor_insert_fg, b.input_cursor_insert_fg),
            input_cursor_insert_bg: color_op(a.input_cursor_insert_bg, b.input_cursor_insert_bg),
            active_fg: color_op(a.active_fg, b.active_fg),
            active_bg: color_op(a.active_bg, b.active_bg),
            border: color_op(a.border, b.border),
            border_active: color_op(a.border_active, b.border_active),
            border_insert: color_op(a.border_insert, b.border_insert),
            popup_bg: color_op(a.popup_bg, b.popup_bg),
            popup_border: color_op(a.popup_border, b.popup_border),
            keybind_key: color_op(a.keybind_key, b.keybind_key),
            keybind_fg: color_op(a.keybind_fg, b.keybind_fg),
            title_bar_bg: color_op(a.title_bar_bg, b.title_bar_bg),
            title_bar_fg: color_op(a.title_bar_fg, b.title_bar_fg),
            tab_fg: color_op(a.tab_fg, b.tab_fg),
            tab_active_fg: color_op(a.tab_active_fg, b.tab_active_fg),
            tab_border: color_op(a.tab_border, b.tab_border),
            status_bar_bg: color_op(a.status_bar_bg, b.status_bar_bg),
            status_bar_fg: color_op(a.status_bar_fg, b.status_bar_fg),
            status_bar_normal_mode_bg: color_op(
                a.status_bar_normal_mode_bg,
                b.status_bar_normal_mode_bg,
            ),
            status_bar_normal_mode_fg: color_op(
                a.status_bar_normal_mode_fg,
                b.status_bar_normal_mode_fg,
            ),
            status_bar_insert_mode_bg: color_op(
                a.status_bar_insert_mode_bg,
                b.status_bar_insert_mode_bg,
            ),
            status_bar_insert_mode_fg: color_op(
                a.status_bar_insert_mode_fg,
                b.status_bar_insert_mode_fg,
            ),
            status_bar_interactive_mode_bg: color_op(
                a.status_bar_interactive_mode_bg,
                b.status_bar_interactive_mode_bg,
            ),
            status_bar_interactive_mode_fg: color_op(
                a.status_bar_interactive_mode_fg,
                b.status_bar_interactive_mode_fg,
            ),
            status_bar_delete_mode_bg: color_op(
                a.status_bar_delete_mode_bg,
                b.status_bar_delete_mode_bg,
            ),
            status_bar_delete_mode_fg: color_op(
                a.status_bar_delete_mode_fg,
                b.status_bar_delete_mode_fg,
            ),
        }
    });

    let modules = user_config.modules.map(|modules| {
        let bcm = base_config.modules.clone();

        let home = modules.home.map(|a| {
            let b = bcm.home.clone();
            HomeModule {
                dashboard_title: a.dashboard_title.unwrap_or(b.dashboard_title),
                dashboard_message: a.dashboard_message.unwrap_or(b.dashboard_message),
            }
        });

        let project_management = modules.project_management.map(|a| {
            let b = bcm.project_management.clone();
            ProjectMangementModule {
                max_lists: a.max_lists.unwrap_or(b.max_lists),
                due_soon_days: a.due_soon_days.unwrap_or(b.due_soon_days),
                completed_char: a.completed_char.unwrap_or(b.completed_char),
                overdue_char: a.overdue_char.unwrap_or(b.overdue_char),
                due_soon_char: a.due_soon_char.unwrap_or(b.due_soon_char),
                in_progress_char: a.in_progress_char.unwrap_or(b.in_progress_char),
                important_char: a.important_char.unwrap_or(b.important_char),
                default_char: a.default_char.unwrap_or(b.default_char),
            }
        });

        ModulesConfig {
            home: home.unwrap_or(bcm.home),
            project_management: project_management.unwrap_or(bcm.project_management),
        }
    });

    let profiles = user_config.profiles.map(|a| {
        a.iter()
            .map(|profile| ProfileConfig::<String> {
                name: profile.name.clone().expect("profile name not provided"),
                config_file: profile
                    .config_file
                    .clone()
                    .expect("profile config file not provided"),
                db_file: profile
                    .db_file
                    .clone()
                    .expect("profile db file not provided"),
                log_file: profile
                    .log_file
                    .clone()
                    .expect("profile log file not provided"),
            })
            .collect()
    });

    Config {
        log_level: user_config
            .log_level
            .clone()
            .unwrap_or(base_config.log_level),
        default_profile: user_config.log_level.clone(),
        colors: colors.unwrap_or(base_config.colors),
        modules: modules.unwrap_or(base_config.modules),
        profiles: profiles.unwrap_or(base_config.profiles),
    }
}

/// Read, parse, and marge the configuration.
pub fn init_config(profile: Option<String>) -> Result<(Config, ProfileConfig)> {
    let base_config = base_config();
    let default_profile = ProfileConfig {
        name: String::from("default"),
        config_file: String::from("config.toml"),
        db_file: String::from("data.db"),
        log_file: String::from("debug.log"),
    };

    let default_config_file = read_config_file(&default_profile.config_file);
    let default_config = match default_config_file? {
        Some(user_config) => merge_config(user_config, base_config.clone()),
        None => base_config.clone(),
    };

    if let Some(profile_name) = profile {
        let profile = default_config
            .profiles
            .iter()
            .find(|p| p.name == profile_name)
            .unwrap_or_else(|| panic!("no profile \"{}\" in config.toml", profile_name))
            .to_owned();

        let profile_config_file = read_config_file(&profile.config_file);
        let base_dev_config = base_dev_config(base_config);
        let profile_config = match profile_config_file? {
            Some(user_config) => merge_config(user_config, base_dev_config),
            None => base_dev_config,
        };
        Ok((profile_config, profile))
    } else {
        Ok((default_config, default_profile))
    }
}
