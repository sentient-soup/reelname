use iced::widget::{column, container, row, stack, Space};
use iced::{task, Element, Length, Subscription, Task, Theme};
use std::collections::HashMap;

use crate::core::parser;
use crate::core::scanner;
use crate::core::tmdb::{TmdbClient, TmdbEpisode, TmdbSeason};
use crate::core::transfer::{self, TransferProgress};
use crate::db::schema::*;
use crate::db::{self, queries, DbConn};
use crate::theme as app_theme;
use crate::ui;

// ── Message enum ──

#[derive(Debug, Clone)]
pub enum Message {
    // Init
    Loaded(Result<InitData, String>),

    // Scan
    ScanRequested,
    ScanCompleted(Result<usize, String>),

    // Match
    MatchRequested,
    MatchCompleted(Result<usize, String>),

    // Groups / Table
    GroupsLoaded(Result<(Vec<GroupWithJobs>, i64), String>),
    GroupClicked(i64),
    GroupLoaded(Result<GroupWithJobs, String>),
    ToggleGroupSelected(i64),
    ToggleSelectAll,
    ClearSelection,
    SortChanged(String),
    PageChanged(i64),

    // Filters
    SearchChanged(String),
    StatusFilterChanged(Option<GroupStatus>),
    MediaTypeFilterChanged(Option<MediaType>),

    // Match Panel
    CloseMatchPanel,
    StartGroupEdit,
    CancelGroupEdit,
    EditTitleChanged(String),
    EditYearChanged(String),
    SaveGroupEdit,
    SkipGroup(i64),
    ConfirmTopMatch(i64),
    ConfirmCompleted(Result<(), String>),
    UseCandidate {
        group_id: i64,
        tmdb_id: i64,
        media_type: MediaType,
    },
    UseCandidateCompleted(Result<(), String>),
    ManualSearchChanged(String),
    ManualSearchSubmit,
    ManualSearchResults(Result<Vec<MatchCandidate>, String>),

    // Episode Resolve
    OpenEpisodeResolve(i64),
    EpisodeSeasonChanged(i64),
    SeasonsLoaded(Result<Vec<TmdbSeason>, String>),
    EpisodesLoaded(Result<Vec<TmdbEpisode>, String>),
    UseEpisode {
        job_id: i64,
        season: i64,
        episode: i64,
        title: String,
    },
    EpisodeUpdated(Result<(), String>),
    CloseEpisodeResolve,

    // Settings
    ToggleSettings,
    SettingChanged(String, String),
    SaveSettings,
    SettingsSaved(Result<(), String>),

    // Transfer
    ToggleTransferDrawer,
    SelectDestination(i64),
    ShowAddDestination,
    HideAddDestination,
    DestFieldChanged(String, String),
    TestConnection,
    TestConnectionResult(Result<String, String>),
    SaveDestination,
    DestinationSaved(Result<i64, String>),
    DeleteDestination(i64),
    DestinationsLoaded(Result<Vec<Destination>, String>),
    StartTransfer,
    TransferProgressUpdate(TransferProgress),
    TransferComplete,

    // Bulk
    BulkAction(String),
    BulkCompleted(Result<(), String>),

    // Toast
    DismissToast(u64),
    TickToasts,

    // Poster
    PosterLoaded(String, Result<Vec<u8>, String>),

    // Keyboard
    KeyPressed(iced::keyboard::Key, iced::keyboard::Modifiers),

    // Tray
    TrayShowWindow,
    TrayQuit,
    WindowCloseRequested(iced::window::Id),
    TrayTick,
}

// ── Init data ──

#[derive(Debug, Clone)]
pub struct InitData {
    pub groups: Vec<GroupWithJobs>,
    pub total_groups: i64,
    pub settings: HashMap<String, String>,
    pub destinations: Vec<Destination>,
}

// ── App state ──

pub struct App {
    pub conn: DbConn,

    // Data
    pub groups: Vec<GroupWithJobs>,
    pub total_groups: i64,
    pub loading: bool,
    pub scanning: bool,

    // Table state
    pub expanded_ids: HashMap<i64, bool>,
    pub selected_ids: HashMap<i64, bool>,
    pub sort_by: String,
    pub sort_dir: String,
    pub page: i64,

    // Filters
    pub search_query: String,
    pub status_filter: Option<GroupStatus>,
    pub media_type_filter: Option<MediaType>,

    // Match panel
    pub active_group_id: Option<i64>,
    pub active_group: Option<GroupWithJobs>,
    pub match_panel_open: bool,
    pub editing_group: bool,
    pub edit_title: String,
    pub edit_year: String,
    pub manual_search_query: String,
    pub manual_search_results: Vec<MatchCandidate>,

    // Episode resolve
    pub episode_resolve_job_id: Option<i64>,
    pub episode_seasons: Vec<TmdbSeason>,
    pub episode_selected_season: i64,
    pub episode_list: Vec<TmdbEpisode>,

    // Settings
    pub settings_open: bool,
    pub settings: HashMap<String, String>,
    pub settings_draft: HashMap<String, String>,

    // Transfer
    pub transfer_drawer_open: bool,
    pub destinations: Vec<Destination>,
    pub selected_destination_id: Option<i64>,
    pub show_add_destination: bool,
    pub dest_form: HashMap<String, String>,
    pub test_connection_result: Option<String>,
    pub active_transfers: Vec<TransferProgress>,
    pub transfer_handle: Option<task::Handle>,

    // Toast
    pub toasts: Vec<crate::ui::toast::Toast>,
    pub next_toast_id: u64,

    // Poster cache
    pub poster_cache: HashMap<String, iced::widget::image::Handle>,
}

impl App {
    fn add_toast(&mut self, message: String, toast_type: crate::ui::toast::ToastType) {
        let id = self.next_toast_id;
        self.next_toast_id += 1;
        self.toasts
            .push(crate::ui::toast::Toast::new(id, message, toast_type));
    }

    fn reload_groups(&self) -> Task<Message> {
        let conn = self.conn.clone();
        let status = self.status_filter;
        let media_type = self.media_type_filter;
        let search = self.search_query.clone();
        let sort_by = self.sort_by.clone();
        let sort_dir = self.sort_dir.clone();
        let page = self.page;

        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || -> Result<_, String> {
                    let (groups, total) = queries::fetch_groups(
                        &conn,
                        status,
                        media_type,
                        Some(&search),
                        &sort_by,
                        &sort_dir,
                        page,
                        50,
                    ).map_err(|e| e.to_string())?;

                    let conn2 = conn.clone();
                    let groups_with_jobs: Vec<GroupWithJobs> = groups
                        .into_iter()
                        .map(|g| {
                            let jobs = queries::fetch_jobs_for_group(&conn2, g.id)
                                .unwrap_or_default()
                                .into_iter()
                                .map(|j| JobWithPreview {
                                    preview_name: None,
                                    job: j,
                                })
                                .collect();
                            let candidates =
                                queries::fetch_candidates_for_group(&conn2, g.id)
                                    .unwrap_or_default();
                            GroupWithJobs {
                                group: g,
                                jobs,
                                candidates,
                            }
                        })
                        .collect();

                    Ok((groups_with_jobs, total))
                })
                .await
                .map_err(|e| format!("Task error: {e}"))?
            },
            Message::GroupsLoaded,
        )
    }
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let db_path = db::db_path();
        let conn = db::open_database(&db_path).expect("Failed to open database");

        let init_conn = conn.clone();
        let init_task = Task::perform(
            async move {
                tokio::task::spawn_blocking(move || -> Result<_, String> {
                    let (groups_raw, total) = queries::fetch_groups(
                        &init_conn,
                        None,
                        None,
                        None,
                        "created_at",
                        "desc",
                        1,
                        50,
                    ).map_err(|e| e.to_string())?;

                    let groups: Vec<GroupWithJobs> = groups_raw
                        .into_iter()
                        .map(|g| {
                            let jobs = queries::fetch_jobs_for_group(&init_conn, g.id)
                                .unwrap_or_default()
                                .into_iter()
                                .map(|j| JobWithPreview {
                                    preview_name: None,
                                    job: j,
                                })
                                .collect();
                            let candidates =
                                queries::fetch_candidates_for_group(&init_conn, g.id)
                                    .unwrap_or_default();
                            GroupWithJobs {
                                group: g,
                                jobs,
                                candidates,
                            }
                        })
                        .collect();

                    let settings_list = queries::fetch_settings(&init_conn).map_err(|e| e.to_string())?;
                    let settings: HashMap<String, String> = settings_list
                        .into_iter()
                        .map(|s| (s.key, s.value))
                        .collect();

                    let destinations = queries::fetch_destinations(&init_conn).map_err(|e| e.to_string())?;

                    Ok(InitData {
                        groups,
                        total_groups: total,
                        settings,
                        destinations,
                    })
                })
                .await
                .map_err(|e| format!("Init error: {e}"))?
            },
            Message::Loaded,
        );

        let app = App {
            conn,
            groups: Vec::new(),
            total_groups: 0,
            loading: true,
            scanning: false,
            expanded_ids: HashMap::new(),
            selected_ids: HashMap::new(),
            sort_by: "created_at".to_string(),
            sort_dir: "desc".to_string(),
            page: 1,
            search_query: String::new(),
            status_filter: None,
            media_type_filter: None,
            active_group_id: None,
            active_group: None,
            match_panel_open: false,
            editing_group: false,
            edit_title: String::new(),
            edit_year: String::new(),
            manual_search_query: String::new(),
            manual_search_results: Vec::new(),
            episode_resolve_job_id: None,
            episode_seasons: Vec::new(),
            episode_selected_season: 1,
            episode_list: Vec::new(),
            settings_open: false,
            settings: HashMap::new(),
            settings_draft: HashMap::new(),
            transfer_drawer_open: false,
            destinations: Vec::new(),
            selected_destination_id: None,
            show_add_destination: false,
            dest_form: HashMap::new(),
            test_connection_result: None,
            active_transfers: Vec::new(),
            transfer_handle: None,
            toasts: Vec::new(),
            next_toast_id: 1,
            poster_cache: HashMap::new(),
        };

        (app, init_task)
    }

    pub fn title(&self) -> String {
        "ReelName".to_string()
    }

    pub fn theme(&self) -> Theme {
        Theme::Dark
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![];

        // Toast auto-dismiss ticker
        if !self.toasts.is_empty() {
            subs.push(iced::time::every(std::time::Duration::from_millis(100)).map(|_| Message::TickToasts));
        }

        // Keyboard events via keyboard::listen()
        subs.push(iced::keyboard::listen().map(|event| {
            match event {
                iced::keyboard::Event::KeyPressed { key, modifiers, .. } => {
                    Message::KeyPressed(key, modifiers)
                }
                _ => Message::TickToasts, // Ignore other keyboard events; reuse a no-op message
            }
        }));

        // Window close → intercept for minimize-to-tray
        subs.push(iced::event::listen_with(|event, _status, id| {
            if let iced::Event::Window(iced::window::Event::CloseRequested) = event {
                Some(Message::WindowCloseRequested(id))
            } else {
                None
            }
        }));

        // Poll system tray events
        if crate::get_tray_menu_ids().is_some() {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(200))
                    .map(|_| Message::TrayTick),
            );
        }

        Subscription::batch(subs)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(Ok(data)) => {
                self.groups = data.groups;
                self.total_groups = data.total_groups;
                self.settings = data.settings.clone();
                self.settings_draft = data.settings;
                self.destinations = data.destinations;
                self.loading = false;
                Task::none()
            }
            Message::Loaded(Err(e)) => {
                self.loading = false;
                self.add_toast(format!("Load error: {e}"), crate::ui::toast::ToastType::Error);
                Task::none()
            }

            // ── Scan ──
            Message::ScanRequested => {
                self.scanning = true;
                let conn = self.conn.clone();
                let scan_path = self.settings.get("scan_path").cloned().unwrap_or_default();

                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            if scan_path.is_empty() {
                                return Err("No scan path configured".to_string());
                            }

                            let path = std::path::Path::new(&scan_path);
                            if !path.exists() {
                                return Err(format!("Scan path does not exist: {scan_path}"));
                            }

                            // Clear existing data
                            queries::delete_all_groups(&conn)
                                .map_err(|e| format!("DB error: {e}"))?;

                            let scanned_groups = scanner::scan_directory_grouped(path);
                            let mut count = 0;

                            for sg in &scanned_groups {
                                // Check if group already exists
                                if queries::group_exists_by_folder(&conn, &sg.folder_path)
                                    .unwrap_or(false)
                                {
                                    continue;
                                }

                                // Parse folder name
                                let parsed = parser::parse_folder_name(&sg.folder_name);

                                // Determine media type from files
                                let has_episodes = sg.files.iter().any(|f| {
                                    f.file_category == FileCategory::Episode
                                        || f.file_category == FileCategory::Special
                                });
                                let media_type = if has_episodes && sg.files.len() > 1 {
                                    MediaType::Tv
                                } else if sg.files.len() == 1
                                    && sg.files[0].file_category == FileCategory::Movie
                                {
                                    MediaType::Movie
                                } else {
                                    MediaType::Unknown
                                };

                                let total_size: i64 =
                                    sg.files.iter().map(|f| f.file_size as i64).sum();

                                let group_id = queries::insert_group(
                                    &conn,
                                    &sg.folder_path,
                                    &sg.folder_name,
                                    parsed.title.as_deref(),
                                    parsed.year,
                                    media_type,
                                    sg.files.len() as i64,
                                    total_size,
                                )
                                .map_err(|e| format!("DB error: {e}"))?;

                                for file in &sg.files {
                                    let parsed_file = parser::parse_file_name(&file.file_name);

                                    let season = file
                                        .detected_season
                                        .or(parsed_file.season);
                                    let episode = parsed_file.episode;

                                    queries::insert_job(
                                        &conn,
                                        group_id,
                                        &file.source_path,
                                        &file.file_name,
                                        file.file_size as i64,
                                        &file.file_extension,
                                        parsed_file.media_type,
                                        file.file_category,
                                        file.extra_type,
                                        parsed_file.title.as_deref(),
                                        parsed_file.year,
                                        season,
                                        episode,
                                        parsed_file.quality.as_deref(),
                                        parsed_file.codec.as_deref(),
                                    )
                                    .map_err(|e| format!("DB error: {e}"))?;
                                }

                                count += 1;
                            }

                            Ok(count)
                        })
                        .await
                        .map_err(|e| format!("Task error: {e}"))?
                    },
                    Message::ScanCompleted,
                )
            }
            Message::ScanCompleted(Ok(count)) => {
                self.scanning = false;
                self.add_toast(
                    format!("Scanned {count} groups"),
                    crate::ui::toast::ToastType::Success,
                );
                self.reload_groups()
            }
            Message::ScanCompleted(Err(e)) => {
                self.scanning = false;
                self.add_toast(format!("Scan error: {e}"), crate::ui::toast::ToastType::Error);
                Task::none()
            }

            // ── Match ──
            Message::MatchRequested => {
                let conn = self.conn.clone();
                let api_key = self.settings.get("tmdb_api_key").cloned().unwrap_or_default();
                let threshold: f64 = self
                    .settings
                    .get("auto_match_threshold")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.85);

                Task::perform(
                    async move {
                        if api_key.is_empty() {
                            return Err("No TMDB API key configured".to_string());
                        }

                        let tmdb = TmdbClient::new(api_key);
                        let groups = tokio::task::spawn_blocking({
                            let conn = conn.clone();
                            move || queries::fetch_scannable_groups(&conn)
                        })
                        .await
                        .map_err(|e| format!("Task error: {e}"))?
                        .map_err(|e| format!("DB error: {e}"))?;

                        let mut matched = 0;
                        for group in &groups {
                            if let Err(e) =
                                crate::core::matcher::match_group(&conn, group, &tmdb, threshold)
                                    .await
                            {
                                tracing::warn!("Match error for group {}: {}", group.id, e);
                            } else {
                                matched += 1;
                            }
                        }

                        Ok(matched)
                    },
                    Message::MatchCompleted,
                )
            }
            Message::MatchCompleted(Ok(count)) => {
                self.add_toast(
                    format!("Matched {count} groups"),
                    crate::ui::toast::ToastType::Success,
                );
                self.reload_groups()
            }
            Message::MatchCompleted(Err(e)) => {
                self.add_toast(format!("Match error: {e}"), crate::ui::toast::ToastType::Error);
                Task::none()
            }

            // ── Groups ──
            Message::GroupsLoaded(Ok((groups, total))) => {
                self.groups = groups;
                self.total_groups = total;
                self.loading = false;
                Task::none()
            }
            Message::GroupsLoaded(Err(e)) => {
                self.loading = false;
                self.add_toast(format!("Load error: {e}"), crate::ui::toast::ToastType::Error);
                Task::none()
            }

            Message::GroupClicked(id) => {
                // Toggle expand
                let was_expanded = self.expanded_ids.get(&id).copied().unwrap_or(false);
                self.expanded_ids.insert(id, !was_expanded);

                // Set active group
                if self.active_group_id == Some(id) {
                    self.active_group_id = None;
                    self.active_group = None;
                    self.match_panel_open = false;
                } else {
                    self.active_group_id = Some(id);
                    self.match_panel_open = true;

                    // Find group in loaded data
                    if let Some(gwj) = self.groups.iter().find(|g| g.group.id == id) {
                        self.active_group = Some(gwj.clone());
                        // Queue poster fetches for candidates
                        let mut tasks = Vec::new();
                        for c in &gwj.candidates {
                            if let Some(path) = &c.poster_path {
                                if !self.poster_cache.contains_key(path) {
                                    let url = format!(
                                        "https://image.tmdb.org/t/p/w92{path}"
                                    );
                                    let path = path.clone();
                                    tasks.push(Task::perform(
                                        async move {
                                            let bytes = reqwest::get(&url)
                                                .await
                                                .map_err(|e| e.to_string())?
                                                .bytes()
                                                .await
                                                .map_err(|e| e.to_string())?;
                                            Ok(bytes.to_vec())
                                        },
                                        move |result| Message::PosterLoaded(path, result),
                                    ));
                                }
                            }
                        }
                        if !tasks.is_empty() {
                            return Task::batch(tasks);
                        }
                    }
                }
                Task::none()
            }

            Message::ToggleGroupSelected(id) => {
                let current = self.selected_ids.get(&id).copied().unwrap_or(false);
                self.selected_ids.insert(id, !current);
                Task::none()
            }
            Message::ToggleSelectAll => {
                let all_selected = !self.groups.is_empty()
                    && self
                        .groups
                        .iter()
                        .all(|g| self.selected_ids.get(&g.group.id).copied().unwrap_or(false));
                if all_selected {
                    self.selected_ids.clear();
                } else {
                    for g in &self.groups {
                        self.selected_ids.insert(g.group.id, true);
                    }
                }
                Task::none()
            }
            Message::ClearSelection => {
                self.selected_ids.clear();
                Task::none()
            }
            Message::SortChanged(field) => {
                if self.sort_by == field {
                    self.sort_dir = if self.sort_dir == "asc" {
                        "desc".to_string()
                    } else {
                        "asc".to_string()
                    };
                } else {
                    self.sort_by = field;
                    self.sort_dir = "asc".to_string();
                }
                self.reload_groups()
            }
            Message::PageChanged(page) => {
                self.page = page;
                self.reload_groups()
            }

            // ── Filters ──
            Message::SearchChanged(query) => {
                self.search_query = query;
                self.page = 1;
                self.reload_groups()
            }
            Message::StatusFilterChanged(status) => {
                self.status_filter = status;
                self.page = 1;
                self.reload_groups()
            }
            Message::MediaTypeFilterChanged(mt) => {
                self.media_type_filter = mt;
                self.page = 1;
                self.reload_groups()
            }

            // ── Match Panel ──
            Message::CloseMatchPanel => {
                self.match_panel_open = false;
                self.active_group = None;
                self.active_group_id = None;
                Task::none()
            }
            Message::StartGroupEdit => {
                if let Some(g) = &self.active_group {
                    self.editing_group = true;
                    self.edit_title = g
                        .group
                        .parsed_title
                        .clone()
                        .unwrap_or_default();
                    self.edit_year = g
                        .group
                        .parsed_year
                        .map(|y| y.to_string())
                        .unwrap_or_default();
                }
                Task::none()
            }
            Message::CancelGroupEdit => {
                self.editing_group = false;
                Task::none()
            }
            Message::EditTitleChanged(t) => {
                self.edit_title = t;
                Task::none()
            }
            Message::EditYearChanged(y) => {
                self.edit_year = y;
                Task::none()
            }
            Message::SaveGroupEdit => {
                if let Some(g) = &self.active_group {
                    let conn = self.conn.clone();
                    let id = g.group.id;
                    let title = self.edit_title.clone();
                    let year: Option<i64> = self.edit_year.parse().ok();
                    self.editing_group = false;

                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                queries::update_group(
                                    &conn,
                                    id,
                                    &[
                                        ("parsed_title", &title as &dyn rusqlite::types::ToSql),
                                        ("parsed_year", &year),
                                    ],
                                )
                                .map_err(|e| format!("DB error: {e}"))
                            })
                            .await
                            .map_err(|e| format!("Task error: {e}"))?
                        },
                        |result| match result {
                            Ok(()) => Message::MatchRequested, // Re-match after edit
                            Err(e) => Message::ConfirmCompleted(Err(e)),
                        },
                    );
                }
                Task::none()
            }
            Message::SkipGroup(id) => {
                let conn = self.conn.clone();
                let status = "skipped".to_string();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            queries::update_group(
                                &conn,
                                id,
                                &[("status", &status as &dyn rusqlite::types::ToSql)],
                            )
                            .map_err(|e| format!("DB error: {e}"))
                        })
                        .await
                        .map_err(|e| format!("Task error: {e}"))?
                    },
                    Message::ConfirmCompleted,
                )
            }
            Message::ConfirmTopMatch(group_id) => {
                let conn = self.conn.clone();
                let status = "confirmed".to_string();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || -> Result<_, String> {
                            queries::update_group(
                                &conn,
                                group_id,
                                &[("status", &status as &dyn rusqlite::types::ToSql)],
                            ).map_err(|e| e.to_string())?;
                            queries::update_jobs_for_group(
                                &conn,
                                group_id,
                                &[("status", &status as &dyn rusqlite::types::ToSql)],
                            ).map_err(|e| e.to_string())?;
                            Ok(())
                        })
                        .await
                        .map_err(|e| format!("Task error: {e}"))?
                    },
                    Message::ConfirmCompleted,
                )
            }
            Message::ConfirmCompleted(Ok(())) => {
                self.add_toast("Updated".to_string(), crate::ui::toast::ToastType::Success);
                self.match_panel_open = false;
                self.active_group = None;
                self.active_group_id = None;
                self.reload_groups()
            }
            Message::ConfirmCompleted(Err(e)) => {
                self.add_toast(format!("Error: {e}"), crate::ui::toast::ToastType::Error);
                Task::none()
            }

            Message::UseCandidate {
                group_id,
                tmdb_id,
                media_type,
            } => {
                // Find candidate in active group
                let candidate = self
                    .active_group
                    .as_ref()
                    .and_then(|g| {
                        g.candidates
                            .iter()
                            .find(|c| c.tmdb_id == tmdb_id)
                            .cloned()
                    })
                    .or_else(|| {
                        self.manual_search_results
                            .iter()
                            .find(|c| c.tmdb_id == tmdb_id)
                            .cloned()
                    });

                if let Some(c) = candidate {
                    let conn = self.conn.clone();
                    let status = "matched".to_string();
                    let title = c.title.clone();
                    let year = c.year;
                    let poster = c.poster_path.clone();
                    let confidence = c.confidence;
                    let mt = media_type.as_str().to_string();

                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || -> Result<_, String> {
                                queries::update_group(
                                    &conn,
                                    group_id,
                                    &[
                                        ("status", &status as &dyn rusqlite::types::ToSql),
                                        ("tmdb_id", &tmdb_id),
                                        ("tmdb_title", &title),
                                        ("tmdb_year", &year),
                                        ("tmdb_poster_path", &poster),
                                        ("match_confidence", &confidence),
                                        ("media_type", &mt),
                                    ],
                                ).map_err(|e| e.to_string())?;
                                queries::update_jobs_for_group(
                                    &conn,
                                    group_id,
                                    &[
                                        ("status", &status as &dyn rusqlite::types::ToSql),
                                        ("tmdb_id", &tmdb_id),
                                        ("tmdb_title", &title),
                                        ("tmdb_year", &year),
                                        ("tmdb_poster_path", &poster),
                                        ("match_confidence", &confidence),
                                        ("media_type", &mt),
                                    ],
                                ).map_err(|e| e.to_string())?;
                                Ok(())
                            })
                            .await
                            .map_err(|e| format!("Task error: {e}"))?
                        },
                        Message::UseCandidateCompleted,
                    )
                } else {
                    Task::none()
                }
            }
            Message::UseCandidateCompleted(Ok(())) => {
                self.add_toast("Match applied".to_string(), crate::ui::toast::ToastType::Success);
                self.reload_groups()
            }
            Message::UseCandidateCompleted(Err(e)) => {
                self.add_toast(format!("Error: {e}"), crate::ui::toast::ToastType::Error);
                Task::none()
            }

            Message::ManualSearchChanged(q) => {
                self.manual_search_query = q;
                Task::none()
            }
            Message::ManualSearchSubmit => {
                let query = self.manual_search_query.clone();
                let api_key = self.settings.get("tmdb_api_key").cloned().unwrap_or_default();
                let group_id = self.active_group_id.unwrap_or(0);

                if query.is_empty() || api_key.is_empty() {
                    return Task::none();
                }

                Task::perform(
                    async move {
                        let tmdb = TmdbClient::new(api_key);
                        let results = tmdb.search_multi(&query, None).await?;

                        let candidates: Vec<MatchCandidate> = results
                            .into_iter()
                            .take(10)
                            .enumerate()
                            .map(|(i, r)| MatchCandidate {
                                id: -(i as i64 + 1), // Negative IDs for unsaved
                                job_id: None,
                                group_id: Some(group_id),
                                tmdb_id: r.id,
                                media_type: MediaType::from_str(r.resolved_media_type()),
                                title: r.display_title().to_string(),
                                year: r.year(),
                                poster_path: r.poster_path,
                                overview: r.overview,
                                confidence: 0.0,
                            })
                            .collect();

                        Ok(candidates)
                    },
                    Message::ManualSearchResults,
                )
            }
            Message::ManualSearchResults(Ok(results)) => {
                self.manual_search_results = results;
                Task::none()
            }
            Message::ManualSearchResults(Err(e)) => {
                self.add_toast(format!("Search error: {e}"), crate::ui::toast::ToastType::Error);
                Task::none()
            }

            // ── Episode Resolve ──
            Message::OpenEpisodeResolve(job_id) => {
                self.episode_resolve_job_id = Some(job_id);

                if let Some(g) = &self.active_group {
                    if let Some(tmdb_id) = g.group.tmdb_id {
                        let api_key =
                            self.settings.get("tmdb_api_key").cloned().unwrap_or_default();

                        // Find current job's season
                        if let Some(jwp) = g.jobs.iter().find(|j| j.job.id == job_id) {
                            self.episode_selected_season =
                                jwp.job.parsed_season.unwrap_or(1);
                        }

                        return Task::perform(
                            async move {
                                let tmdb = TmdbClient::new(api_key);
                                tmdb.get_seasons(tmdb_id).await
                            },
                            Message::SeasonsLoaded,
                        );
                    }
                }
                Task::none()
            }
            Message::SeasonsLoaded(Ok(seasons)) => {
                self.episode_seasons = seasons;
                // Load episodes for selected season
                if let Some(g) = &self.active_group {
                    if let Some(tmdb_id) = g.group.tmdb_id {
                        let api_key =
                            self.settings.get("tmdb_api_key").cloned().unwrap_or_default();
                        let season = self.episode_selected_season;
                        return Task::perform(
                            async move {
                                let tmdb = TmdbClient::new(api_key);
                                let detail = tmdb.get_season_detail(tmdb_id, season).await?;
                                Ok(detail.episodes)
                            },
                            Message::EpisodesLoaded,
                        );
                    }
                }
                Task::none()
            }
            Message::SeasonsLoaded(Err(e)) => {
                self.add_toast(format!("Seasons error: {e}"), crate::ui::toast::ToastType::Error);
                Task::none()
            }
            Message::EpisodeSeasonChanged(season) => {
                self.episode_selected_season = season;
                if let Some(g) = &self.active_group {
                    if let Some(tmdb_id) = g.group.tmdb_id {
                        let api_key =
                            self.settings.get("tmdb_api_key").cloned().unwrap_or_default();
                        return Task::perform(
                            async move {
                                let tmdb = TmdbClient::new(api_key);
                                let detail = tmdb.get_season_detail(tmdb_id, season).await?;
                                Ok(detail.episodes)
                            },
                            Message::EpisodesLoaded,
                        );
                    }
                }
                Task::none()
            }
            Message::EpisodesLoaded(Ok(episodes)) => {
                self.episode_list = episodes;
                Task::none()
            }
            Message::EpisodesLoaded(Err(e)) => {
                self.add_toast(
                    format!("Episodes error: {e}"),
                    crate::ui::toast::ToastType::Error,
                );
                Task::none()
            }
            Message::UseEpisode {
                job_id,
                season,
                episode,
                title,
            } => {
                let conn = self.conn.clone();
                let file_category = if season == 0 {
                    "special".to_string()
                } else {
                    "episode".to_string()
                };

                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            queries::update_job(
                                &conn,
                                job_id,
                                &[
                                    ("parsed_season", &season as &dyn rusqlite::types::ToSql),
                                    ("parsed_episode", &episode),
                                    ("tmdb_episode_title", &title),
                                    ("file_category", &file_category),
                                ],
                            )
                            .map_err(|e| format!("DB error: {e}"))
                        })
                        .await
                        .map_err(|e| format!("Task error: {e}"))?
                    },
                    Message::EpisodeUpdated,
                )
            }
            Message::EpisodeUpdated(Ok(())) => {
                self.add_toast("Episode updated".to_string(), crate::ui::toast::ToastType::Success);
                self.episode_resolve_job_id = None;
                self.reload_groups()
            }
            Message::EpisodeUpdated(Err(e)) => {
                self.add_toast(format!("Error: {e}"), crate::ui::toast::ToastType::Error);
                Task::none()
            }
            Message::CloseEpisodeResolve => {
                self.episode_resolve_job_id = None;
                Task::none()
            }

            // ── Settings ──
            Message::ToggleSettings => {
                self.settings_open = !self.settings_open;
                if self.settings_open {
                    self.settings_draft = self.settings.clone();
                }
                Task::none()
            }
            Message::SettingChanged(key, value) => {
                self.settings_draft.insert(key, value);
                Task::none()
            }
            Message::SaveSettings => {
                let conn = self.conn.clone();
                let draft = self.settings_draft.clone();

                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let pairs: Vec<(&str, &str)> = draft
                                .iter()
                                .map(|(k, v)| (k.as_str(), v.as_str()))
                                .collect();
                            queries::update_settings(&conn, &pairs)
                                .map_err(|e| format!("DB error: {e}"))
                        })
                        .await
                        .map_err(|e| format!("Task error: {e}"))?
                    },
                    Message::SettingsSaved,
                )
            }
            Message::SettingsSaved(Ok(())) => {
                self.settings = self.settings_draft.clone();
                self.settings_open = false;
                self.add_toast(
                    "Settings saved".to_string(),
                    crate::ui::toast::ToastType::Success,
                );
                Task::none()
            }
            Message::SettingsSaved(Err(e)) => {
                self.add_toast(format!("Save error: {e}"), crate::ui::toast::ToastType::Error);
                Task::none()
            }

            // ── Transfer ──
            Message::ToggleTransferDrawer => {
                self.transfer_drawer_open = !self.transfer_drawer_open;
                if self.transfer_drawer_open {
                    // Reload destinations
                    let conn = self.conn.clone();
                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                queries::fetch_destinations(&conn)
                                    .map_err(|e| format!("DB error: {e}"))
                            })
                            .await
                            .map_err(|e| format!("Task error: {e}"))?
                        },
                        Message::DestinationsLoaded,
                    );
                }
                Task::none()
            }
            Message::SelectDestination(id) => {
                self.selected_destination_id = Some(id);
                Task::none()
            }
            Message::ShowAddDestination => {
                self.show_add_destination = true;
                self.dest_form.clear();
                self.dest_form.insert("type".to_string(), "local".to_string());
                self.dest_form.insert("ssh_port".to_string(), "22".to_string());
                self.test_connection_result = None;
                Task::none()
            }
            Message::HideAddDestination => {
                self.show_add_destination = false;
                Task::none()
            }
            Message::DestFieldChanged(field, value) => {
                self.dest_form.insert(field, value);
                Task::none()
            }
            Message::TestConnection => {
                self.test_connection_result = Some("Testing...".to_string());
                let host = self.dest_form.get("ssh_host").cloned().unwrap_or_default();
                let port: u16 = self
                    .dest_form
                    .get("ssh_port")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(22);
                let user = self.dest_form.get("ssh_user").cloned().unwrap_or_default();
                let key_path = self.dest_form.get("ssh_key_path").cloned().unwrap_or_default();
                let passphrase = self.dest_form.get("ssh_key_passphrase").cloned();

                Task::perform(
                    async move {
                        transfer::test_ssh_connection(
                            &host,
                            port,
                            &user,
                            &key_path,
                            passphrase.as_deref(),
                        )
                        .await
                    },
                    Message::TestConnectionResult,
                )
            }
            Message::TestConnectionResult(result) => {
                self.test_connection_result = Some(match result {
                    Ok(msg) => msg,
                    Err(e) => e,
                });
                Task::none()
            }
            Message::SaveDestination => {
                let conn = self.conn.clone();
                let form = self.dest_form.clone();

                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let name = form.get("name").map(|s| s.as_str()).unwrap_or("");
                            let dest_type = DestinationType::from_str(
                                form.get("type").map(|s| s.as_str()).unwrap_or("local"),
                            );
                            let base_path =
                                form.get("base_path").map(|s| s.as_str()).unwrap_or("");

                            if name.is_empty() || base_path.is_empty() {
                                return Err("Name and base path are required".to_string());
                            }

                            queries::insert_destination(
                                &conn,
                                name,
                                dest_type,
                                base_path,
                                form.get("ssh_host").map(|s| s.as_str()),
                                form.get("ssh_port")
                                    .and_then(|s| s.parse().ok()),
                                form.get("ssh_user").map(|s| s.as_str()),
                                form.get("ssh_key_path").map(|s| s.as_str()),
                                form.get("ssh_key_passphrase").map(|s| s.as_str()),
                            )
                            .map_err(|e| format!("DB error: {e}"))
                        })
                        .await
                        .map_err(|e| format!("Task error: {e}"))?
                    },
                    Message::DestinationSaved,
                )
            }
            Message::DestinationSaved(Ok(_id)) => {
                self.show_add_destination = false;
                self.add_toast(
                    "Destination added".to_string(),
                    crate::ui::toast::ToastType::Success,
                );
                // Reload destinations
                let conn = self.conn.clone();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            queries::fetch_destinations(&conn)
                                .map_err(|e| format!("DB error: {e}"))
                        })
                        .await
                        .map_err(|e| format!("Task error: {e}"))?
                    },
                    Message::DestinationsLoaded,
                )
            }
            Message::DestinationSaved(Err(e)) => {
                self.add_toast(format!("Error: {e}"), crate::ui::toast::ToastType::Error);
                Task::none()
            }
            Message::DeleteDestination(id) => {
                let conn = self.conn.clone();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            queries::delete_destination(&conn, id)
                                .map_err(|e| format!("DB error: {e}"))
                        })
                        .await
                        .map_err(|e| format!("Task error: {e}"))?
                    },
                    |_| Message::ToggleTransferDrawer, // Reload
                )
            }
            Message::DestinationsLoaded(Ok(dests)) => {
                self.destinations = dests;
                Task::none()
            }
            Message::DestinationsLoaded(Err(e)) => {
                self.add_toast(format!("Error: {e}"), crate::ui::toast::ToastType::Error);
                Task::none()
            }

            Message::StartTransfer => {
                if let Some(dest_id) = self.selected_destination_id {
                    let selected: Vec<i64> = self
                        .selected_ids
                        .iter()
                        .filter(|&(_, &v)| v)
                        .map(|(&k, _)| k)
                        .collect();

                    if selected.is_empty() {
                        self.add_toast(
                            "No groups selected".to_string(),
                            crate::ui::toast::ToastType::Warning,
                        );
                        return Task::none();
                    }

                    let conn = self.conn.clone();
                    // Fetch confirmed jobs for selected groups
                    let job_ids: Vec<i64> = match queries::fetch_confirmed_jobs(&conn, &selected) {
                        Ok(jobs) => jobs.into_iter().map(|j| j.id).collect(),
                        Err(e) => {
                            self.add_toast(
                                format!("Error: {e}"),
                                crate::ui::toast::ToastType::Error,
                            );
                            return Task::none();
                        }
                    };

                    if job_ids.is_empty() {
                        self.add_toast(
                            "No confirmed jobs to transfer".to_string(),
                            crate::ui::toast::ToastType::Warning,
                        );
                        return Task::none();
                    }

                    self.active_transfers.clear();
                    let rx = transfer::start_transfers(conn, job_ids, dest_id);

                    // Convert mpsc receiver to a stream of Messages
                    let stream = futures::stream::unfold(rx, |mut rx| async move {
                        let progress = rx.recv().await?;
                        Some((Message::TransferProgressUpdate(progress), rx))
                    });

                    let (task, handle) = Task::stream(stream)
                        .chain(Task::done(Message::TransferComplete))
                        .abortable();
                    self.transfer_handle = Some(handle);
                    return task;
                }
                Task::none()
            }
            Message::TransferProgressUpdate(progress) => {
                // Update or insert transfer progress
                if let Some(existing) = self
                    .active_transfers
                    .iter_mut()
                    .find(|t| t.job_id == progress.job_id)
                {
                    *existing = progress;
                } else {
                    self.active_transfers.push(progress);
                }
                Task::none()
            }
            Message::TransferComplete => {
                self.transfer_handle = None;
                self.add_toast(
                    "Transfers completed".to_string(),
                    crate::ui::toast::ToastType::Success,
                );
                self.reload_groups()
            }

            // ── Bulk ──
            Message::BulkAction(action) => {
                let conn = self.conn.clone();
                let selected: Vec<i64> = self
                    .selected_ids
                    .iter()
                    .filter(|&(_, &v)| v)
                    .map(|(&k, _)| k)
                    .collect();

                if selected.is_empty() {
                    return Task::none();
                }

                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || -> Result<_, String> {
                            for id in &selected {
                                match action.as_str() {
                                    "confirm" => {
                                        let s = "confirmed".to_string();
                                        queries::update_group(
                                            &conn,
                                            *id,
                                            &[("status", &s as &dyn rusqlite::types::ToSql)],
                                        ).map_err(|e| e.to_string())?;
                                        queries::update_jobs_for_group(
                                            &conn,
                                            *id,
                                            &[("status", &s as &dyn rusqlite::types::ToSql)],
                                        ).map_err(|e| e.to_string())?;
                                    }
                                    "skip" => {
                                        let s = "skipped".to_string();
                                        queries::update_group(
                                            &conn,
                                            *id,
                                            &[("status", &s as &dyn rusqlite::types::ToSql)],
                                        ).map_err(|e| e.to_string())?;
                                    }
                                    "delete" => {
                                        queries::delete_group(&conn, *id).map_err(|e| e.to_string())?;
                                    }
                                    "rematch" => {
                                        let s = "scanned".to_string();
                                        queries::update_group(
                                            &conn,
                                            *id,
                                            &[("status", &s as &dyn rusqlite::types::ToSql)],
                                        ).map_err(|e| e.to_string())?;
                                    }
                                    _ => {}
                                }
                            }
                            Ok(())
                        })
                        .await
                        .map_err(|e| format!("Task error: {e}"))?
                    },
                    Message::BulkCompleted,
                )
            }
            Message::BulkCompleted(Ok(())) => {
                self.selected_ids.clear();
                self.add_toast(
                    "Bulk action completed".to_string(),
                    crate::ui::toast::ToastType::Success,
                );
                self.reload_groups()
            }
            Message::BulkCompleted(Err(e)) => {
                self.add_toast(format!("Error: {e}"), crate::ui::toast::ToastType::Error);
                Task::none()
            }

            // ── Toast ──
            Message::DismissToast(id) => {
                self.toasts.retain(|t| t.id != id);
                Task::none()
            }
            Message::TickToasts => {
                self.toasts.retain(|t| !t.is_expired());
                Task::none()
            }

            // ── Poster ──
            Message::PosterLoaded(path, Ok(bytes)) => {
                let handle = iced::widget::image::Handle::from_bytes(bytes);
                self.poster_cache.insert(path, handle);
                Task::none()
            }
            Message::PosterLoaded(_, Err(_)) => Task::none(),

            // ── Keyboard ──
            Message::KeyPressed(key, modifiers) => {
                use iced::keyboard::Key;

                match key {
                    Key::Named(iced::keyboard::key::Named::Escape) => {
                        if self.episode_resolve_job_id.is_some() {
                            self.episode_resolve_job_id = None;
                        } else if self.settings_open {
                            self.settings_open = false;
                        } else if self.show_add_destination {
                            self.show_add_destination = false;
                        } else if self.match_panel_open {
                            self.match_panel_open = false;
                            self.active_group = None;
                            self.active_group_id = None;
                        }
                        Task::none()
                    }
                    Key::Character(c) => {
                        let c = c.as_str();
                        if modifiers.command() && c == "a" {
                            // Select all
                            for g in &self.groups {
                                self.selected_ids.insert(g.group.id, true);
                            }
                            Task::none()
                        } else if modifiers.command() && c == "d" {
                            self.selected_ids.clear();
                            Task::none()
                        } else if c == "r" && !modifiers.command() {
                            self.reload_groups()
                        } else if c == "s" && !modifiers.command() {
                            return self.update(Message::ScanRequested);
                        } else if c == "," {
                            self.settings_open = !self.settings_open;
                            Task::none()
                        } else {
                            Task::none()
                        }
                    }
                    _ => Task::none(),
                }
            }

            // ── Tray ──
            Message::TrayTick => {
                if let Some(ids) = crate::get_tray_menu_ids() {
                    match crate::tray::poll_tray_event(ids) {
                        Some(crate::tray::TrayAction::ShowWindow) => {
                            return iced::window::latest()
                                .and_then(|id| {
                                    Task::batch([
                                        iced::window::minimize(id, false),
                                        iced::window::gain_focus(id),
                                    ])
                                });
                        }
                        Some(crate::tray::TrayAction::Quit) => {
                            return iced::exit();
                        }
                        None => {}
                    }
                }
                Task::none()
            }
            Message::TrayShowWindow => {
                iced::window::latest()
                    .and_then(|id| {
                        Task::batch([
                            iced::window::minimize(id, false),
                            iced::window::gain_focus(id),
                        ])
                    })
            }
            Message::TrayQuit => iced::exit(),
            Message::WindowCloseRequested(id) => {
                if crate::get_tray_menu_ids().is_some() {
                    // Tray exists: minimize to tray instead of closing
                    iced::window::minimize(id, true)
                } else {
                    // No tray: actually close
                    iced::window::close(id)
                }
            }

            _ => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        // Calculate stats
        let total_files: i64 = self.groups.iter().map(|g| g.group.total_file_count).sum();
        let selected_count = self.selected_ids.values().filter(|&&v| v).count();

        // Header
        let header = ui::header::header_bar(
            self.total_groups,
            total_files,
            selected_count,
            self.scanning,
        );

        // Filters
        let filters = ui::filters::filters_bar(
            &self.search_query,
            self.status_filter,
            self.media_type_filter,
            selected_count,
        );

        // Table
        let table = ui::queue_table::queue_table(
            &self.groups,
            &self.expanded_ids,
            &self.selected_ids,
            self.active_group_id,
            &self.sort_by,
            &self.sort_dir,
        );

        // Pagination
        let pagination = ui::pagination::pagination_bar(self.page, self.total_groups);

        // Main content area (table + optional match panel)
        let main_content: Element<'_, Message> = if self.match_panel_open {
            if let Some(group) = &self.active_group {
                row![
                    column![table, pagination].width(Length::Fill),
                    ui::match_panel::match_panel(
                        group,
                        &self.manual_search_query,
                        &self.manual_search_results,
                        self.editing_group,
                        &self.edit_title,
                        &self.edit_year,
                        &self.poster_cache,
                    ),
                ]
                .into()
            } else {
                column![table, pagination].width(Length::Fill).into()
            }
        } else {
            column![table, pagination].width(Length::Fill).into()
        };

        // Transfer drawer
        let transfer: Element<'_, Message> = if self.transfer_drawer_open {
            let confirmed_count = self
                .selected_ids
                .iter()
                .filter(|&(_, &v)| v)
                .filter(|(id, _)| {
                    self.groups
                        .iter()
                        .any(|g| g.group.id == **id && g.group.status == GroupStatus::Confirmed)
                })
                .count();

            ui::transfer_drawer::transfer_drawer(
                &self.destinations,
                self.selected_destination_id,
                confirmed_count,
                &self.active_transfers,
                self.show_add_destination,
            )
        } else {
            Space::new().height(0).into()
        };

        // Base layout
        let base = container(
            column![header, filters, main_content, transfer]
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_: &Theme| container::Style {
            background: Some(app_theme::BG_PRIMARY.into()),
            ..Default::default()
        });

        // Stack for overlays (modals, toasts)
        let mut layers: Vec<Element<'_, Message>> = vec![base.into()];

        // Settings modal
        if self.settings_open {
            layers.push(ui::settings_modal::settings_modal(
                self.settings_draft.get("scan_path").map(|s| s.as_str()).unwrap_or(""),
                self.settings_draft.get("tmdb_api_key").map(|s| s.as_str()).unwrap_or(""),
                self.settings_draft
                    .get("auto_match_threshold")
                    .map(|s| s.as_str())
                    .unwrap_or("0.85"),
                self.settings_draft.get("data_dir").map(|s| s.as_str()).unwrap_or(""),
                self.settings_draft
                    .get("naming_preset")
                    .map(|s| s.as_str())
                    .unwrap_or("jellyfin"),
                self.settings_draft
                    .get("specials_folder_name")
                    .map(|s| s.as_str())
                    .unwrap_or("Specials"),
                self.settings_draft
                    .get("extras_folder_name")
                    .map(|s| s.as_str())
                    .unwrap_or("Extras"),
            ));
        }

        // Add destination modal
        if self.show_add_destination {
            layers.push(ui::transfer_drawer::add_destination_modal(
                self.dest_form.get("name").map(|s| s.as_str()).unwrap_or(""),
                self.dest_form.get("type").map(|s| s.as_str()).unwrap_or("local"),
                self.dest_form.get("base_path").map(|s| s.as_str()).unwrap_or(""),
                self.dest_form.get("ssh_host").map(|s| s.as_str()).unwrap_or(""),
                self.dest_form.get("ssh_port").map(|s| s.as_str()).unwrap_or("22"),
                self.dest_form.get("ssh_user").map(|s| s.as_str()).unwrap_or(""),
                self.dest_form.get("ssh_key_path").map(|s| s.as_str()).unwrap_or(""),
                self.dest_form
                    .get("ssh_key_passphrase")
                    .map(|s| s.as_str())
                    .unwrap_or(""),
                self.test_connection_result.as_deref(),
            ));
        }

        // Episode resolve modal
        if let Some(job_id) = self.episode_resolve_job_id {
            let (current_season, current_episode) = self
                .active_group
                .as_ref()
                .and_then(|g| g.jobs.iter().find(|j| j.job.id == job_id))
                .map(|j| (j.job.parsed_season, j.job.parsed_episode))
                .unwrap_or((None, None));

            layers.push(ui::episode_resolve_modal::episode_resolve_modal(
                job_id,
                &self.episode_seasons,
                self.episode_selected_season,
                &self.episode_list,
                current_season,
                current_episode,
            ));
        }

        // Toasts
        if !self.toasts.is_empty() {
            let toast_view = ui::toast::toast_container(&self.toasts);
            layers.push(
                container(toast_view)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Right)
                    .align_y(iced::alignment::Vertical::Bottom)
                    .into(),
            );
        }

        stack(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
