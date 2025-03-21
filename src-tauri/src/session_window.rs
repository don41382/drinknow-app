use crate::alert::Alert;
use crate::model::settings::WelcomeWizardMode;
use crate::{
    countdown_timer, feedback_window, tracking, updater_window, welcome_window,
    CountdownTimerState, LicenseManagerState, SettingsManagerState, SettingsSystemState,
    TrackingState,
};
use anyhow::{anyhow, Error};
use core::clone::Clone;
use log::info;
use tauri::{AppHandle, EventId, Manager, State, WebviewWindowBuilder, Wry};
use tauri_specta::Event;

use crate::feedback_window::FeedbackDisplay;
use crate::model::event::SessionStartEvent;
use crate::model::session::{DrinkCharacter, SipSize};

pub const WINDOW_LABEL: &'static str = "session";

pub fn init(app: &AppHandle<Wry>) -> Result<EventId, anyhow::Error> {
    let app_handle = app.clone();
    build_session_window(app)?;
    let id = countdown_timer::CountdownEvent::listen(app, move |status| {
        if status.payload.status == countdown_timer::TimerStatus::Finished {
            let app_handle_start = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                let timer = app_handle_start.app_handle().state::<CountdownTimerState>();
                timer.restart();

                show_session(&app_handle_start.app_handle(), None)
                    .await
                    .unwrap();
            });
        }
    });
    Ok(id)
}

#[specta::specta]
#[tauri::command]
pub async fn start_session(
    app: AppHandle,
    drink_settings: Option<SessionStartEvent>,
    timer: State<'_, CountdownTimerState>,
) -> Result<(), ()> {
    let demo_mode = drink_settings.as_ref().map(|s| s.demo_mode).unwrap_or(false);
    if !demo_mode {
        timer.restart();
    }

    show_session(&app, drink_settings)
        .await
        .unwrap_or_else(|err| {
            app.alert(
                "Can't start session",
                "There was an error while trying to start the session.",
                Some(err),
                false,
            );
        });
    Ok(())
}

pub async fn show_session(
    app: &AppHandle<Wry>,
    overwrite_settings: Option<SessionStartEvent>,
) -> Result<(), anyhow::Error> {
    let license_active = app
        .state::<LicenseManagerState>()
        .get_status(&app.app_handle(), false, false)
        .await
        .map(|s| s.status.is_active())
        .map_err(|err| anyhow!(err))
        .unwrap_or_else(|err| {
            app.alert(
                "Unable to access license server",
                "We are sorry, but we have trouble accessing the license server.",
                Some(err),
                true,
            );
            false
        });

    let demo_mode = overwrite_settings
        .as_ref()
        .map(|s| s.demo_mode.clone())
        .unwrap_or(false);
    if demo_mode || license_active {
        if !demo_mode {
            // stop current running timer
            info!("increase session counter");
            {
                let settings_system = app.state::<SettingsSystemState>();
                let mut settings_system = settings_system
                    .lock()
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                settings_system.increase_session_count(&app);
            }

            // send tracking event
            info!("start session window: send tracking");
            app.state::<TrackingState>()
                .send_tracking(tracking::Event::DrinkReminder)
                .await;
        }

        let user_settings = app
            .state::<SettingsManagerState>()
            .get_settings()
            .map(|s| s.user);

        let session_start: SessionStartEvent = overwrite_settings
            .or_else(|| {
                // Attempt to get from user settings if overwrite_settings is None
                user_settings
                    .as_ref()
                    .map(|user_settings| SessionStartEvent {
                        sip_size: user_settings.sip_size.clone(),
                        selected_drink_character: user_settings.character.clone(),
                        demo_mode: false,
                    })
            })
            .unwrap_or_else(|| {
                // Provide a default SessionStartEvent if both overwrite_settings and user settings are None
                SessionStartEvent {
                    sip_size: SipSize::BigSip,
                    selected_drink_character: DrinkCharacter::YoungWoman,
                    demo_mode: false,
                }
            });

        if let Some(_window) = app.get_webview_window(WINDOW_LABEL) {
            info!("start session window: send event");
            session_start.emit(app.app_handle())?;
        } else {
            app.alert(
                "Session Window Missing",
                "I am sorry, this should not happen. Please contact Rocket Solutions",
                None,
                false,
            );
        }
    } else {
        welcome_window::show(
            app.app_handle(),
            &app.state::<TrackingState>().device_id(),
            WelcomeWizardMode::OnlyPayment,
        )
        .await?;
    }

    Ok(())
}

fn build_session_window(app: &AppHandle) -> Result<(), Error> {
    info!("start session window: create new window");
    let window =
        WebviewWindowBuilder::new(app, WINDOW_LABEL, tauri::WebviewUrl::App("/session".into()))
            .title("Just Drink! Session")
            .transparent(true)
            .visible(false)
            .always_on_top(true)
            .decorations(false)
            .maximized(true)
            .skip_taskbar(false)
            .accept_first_mouse(true)
            .visible_on_all_workspaces(true)
            .focused(false)
            .resizable(false);

    info!("start session window: build");
    let window = window.build()?;
    window.set_ignore_cursor_events(true)?;

    Ok(())
}

pub(crate) fn days_between(
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
) -> i64 {
    let duration: chrono::Duration = end - start;
    duration.num_days() + 1
}

#[specta::specta]
#[tauri::command]
pub async fn end_session(
    app: AppHandle,
    settings_system: State<'_, SettingsSystemState>,
    demo_mode: bool,
) -> Result<(), String> {
    info!("end reminder session");
    hide_window(&app)?;

    if !demo_mode {
        let ask_for_feedback = {
            let ss = settings_system
                .lock()
                .expect("settings_system should not be locked");
            ss.should_show_feedback()
        };

        let updater_visible = updater_window::show_if_update_available(&app, false, false).await;

        if ask_for_feedback && !updater_visible {
            feedback_window::show(&app).expect("unable to show feedback window");
        }
    }

    Ok(())
}

pub fn hide_window(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        window
            .hide()
            .map_err(|err| format!("window can't be closed: {}", err))?;
    }
    Ok(())
}
