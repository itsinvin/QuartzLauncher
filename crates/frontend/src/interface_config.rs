use std::{cmp::Ordering, io::Write, path::Path, sync::Arc, time::Duration};

use bridge::instance::InstanceContentSummary;
use gpui::{App, SharedString, Task};
use rand::RngCore;
use schema::{curseforge::{CurseforgeClassId, CurseforgeHit}, modrinth::{ModrinthHit, ModrinthProjectType}};
use serde::{Deserialize, Serialize};

use crate::{pages::instance::instance_page::InstanceSubpageType, ui::PageType};

struct InterfaceConfigHolder {
    config: InterfaceConfig,
    write_task: Option<Task<()>>,
    path: Arc<Path>,
}

impl gpui::Global for InterfaceConfigHolder {}

#[derive(Debug, Serialize, Deserialize)]
pub struct InterfaceConfig {
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub active_theme: SharedString,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub main_window_bounds: WindowBounds,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub sidebar_width: f32,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub main_page: PageType,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub page_path: Arc<[PageType]>,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub quick_delete_mods: bool,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub quick_delete_instance: bool,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub instance_mods_sort_key: InstanceContentSortKey,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub instance_mods_sort_enabled_first: bool,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub instance_resourcepacks_sort_key: InstanceContentSortKey,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub instance_resourcepacks_sort_enabled_first: bool,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub instance_shaders_sort_key: InstanceContentSortKey,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub instance_shaders_sort_enabled_first: bool,
    #[serde(default = "schema::default_true", deserialize_with = "schema::try_deserialize")]
    pub content_install_latest: bool,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub content_filter_version: bool,
    #[serde(default = "default_modrinth_project_type", deserialize_with = "schema::try_deserialize")]
    pub modrinth_page_project_type: ModrinthProjectType,
    #[serde(default = "default_curseforge_class_id", deserialize_with = "schema::try_deserialize")]
    pub curseforge_page_class_id: CurseforgeClassId,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub hide_main_window_on_launch: bool,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub quit_on_main_closed: bool,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub use_os_titlebar: bool,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub hide_usernames: bool,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub hide_skins: bool,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub hide_server_addresses: bool,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub show_snapshots_in_create_instance: bool,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub instances_view_mode: InstancesViewMode,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub instance_subpage: InstanceSubpageType,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub collapse_capes_in_skins_page: bool,
    #[serde(default = "schema::default_true", deserialize_with = "schema::try_deserialize")]
    pub skin_list_show_3d: bool,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub modrinth_favorites: Vec<ModrinthFavorite>,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub curseforge_favorites: Vec<CurseforgeFavorite>,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub modrinth_favorites_only: bool,
    #[serde(default, deserialize_with = "schema::try_deserialize")]
    pub curseforge_favorites_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModrinthFavorite {
    pub project_id: String,
    pub title: String,
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub project_type: ModrinthProjectType,
}

impl ModrinthFavorite {
    pub fn from_hit(hit: &ModrinthHit) -> Self {
        Self {
            project_id: hit.project_id.to_string(),
            title: hit.title.as_deref().unwrap_or("").to_string(),
            author: hit.author.to_string(),
            description: hit.description.as_deref().unwrap_or("").to_string(),
            icon_url: hit.icon_url.as_deref().map(str::to_string),
            downloads: hit.downloads,
            project_type: hit.project_type,
        }
    }

    pub fn to_hit(&self) -> ModrinthHit {
        ModrinthHit {
            title: Some(Arc::from(self.title.as_str())),
            description: Some(Arc::from(self.description.as_str())),
            client_side: None,
            server_side: None,
            project_type: self.project_type,
            downloads: self.downloads,
            icon_url: self.icon_url.as_deref().map(Arc::from),
            project_id: Arc::from(self.project_id.as_str()),
            author: Arc::from(self.author.as_str()),
            display_categories: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CurseforgeFavorite {
    pub id: u32,
    pub name: String,
    pub summary: String,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    pub download_count: u64,
    pub class_id: u32,
}

impl CurseforgeFavorite {
    pub fn from_hit(hit: &CurseforgeHit) -> Self {
        Self {
            id: hit.id,
            name: hit.name.to_string(),
            summary: hit.summary.to_string(),
            thumbnail_url: hit.logo.as_ref().map(|l| l.thumbnail_url.to_string()),
            download_count: hit.download_count,
            class_id: hit.class_id.unwrap_or(0),
        }
    }

    pub fn to_hit(&self) -> CurseforgeHit {
        use schema::curseforge::{CurseforgeModAsset, FileIndex};
        CurseforgeHit {
            id: self.id,
            game_id: 432,
            name: Arc::from(self.name.as_str()),
            slug: Arc::from(""),
            summary: Arc::from(self.summary.as_str()),
            download_count: self.download_count,
            class_id: Some(self.class_id),
            logo: self.thumbnail_url.as_deref().map(|url| CurseforgeModAsset {
                thumbnail_url: Arc::from(url),
            }),
            authors: Arc::new([]),
            categories: Arc::new([]),
            latest_files_indexes: Arc::new([] as [FileIndex; 0]),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, strum::EnumIter)]
#[serde(rename_all = "lowercase")]
pub enum InstanceContentSortKey {
    #[default]
    Name,
    ModId,
    Filename,
    ModifiedTime,
    FileSize,
}

impl InstanceContentSortKey {
    pub fn name(self) -> SharedString {
        match self {
            InstanceContentSortKey::Name => "Name".into(),
            InstanceContentSortKey::ModId => "Mod Id".into(),
            InstanceContentSortKey::Filename => "Filename".into(),
            InstanceContentSortKey::ModifiedTime => "Modified Time".into(),
            InstanceContentSortKey::FileSize => "Filesize".into(),
        }
    }

    pub fn compare(self, a: &InstanceContentSummary, b: &InstanceContentSummary) -> Ordering {
        match self {
            InstanceContentSortKey::Name => {
                let name_a = a.content_summary.name.as_deref().or(a.content_summary.id.as_deref()).unwrap_or(&*a.filename);
                let name_b = b.content_summary.name.as_deref().or(b.content_summary.id.as_deref()).unwrap_or(&*b.filename);
                lexical_sort::natural_lexical_cmp(name_a, name_b)
            },
            InstanceContentSortKey::ModId => {
                let name_a = a.content_summary.id.as_deref().or(a.content_summary.name.as_deref()).unwrap_or(&*a.filename);
                let name_b = b.content_summary.id.as_deref().or(b.content_summary.name.as_deref()).unwrap_or(&*b.filename);
                lexical_sort::natural_lexical_cmp(name_a, name_b)
            },
            InstanceContentSortKey::Filename => {
                let name_a = &*a.filename;
                let name_b = &*b.filename;
                lexical_sort::natural_lexical_cmp(name_a, name_b)
            },
            InstanceContentSortKey::ModifiedTime => {
                a.modified_unix_ms.cmp(&b.modified_unix_ms).reverse()
            },
            InstanceContentSortKey::FileSize => {
                a.content_summary.filesize.unwrap_or(0).cmp(&b.content_summary.filesize.unwrap_or(0)).reverse()
            },
        }
    }
}

fn default_modrinth_project_type() -> ModrinthProjectType {
    ModrinthProjectType::Mod
}

fn default_curseforge_class_id() -> CurseforgeClassId {
    CurseforgeClassId::Mod
}

impl Default for InterfaceConfig {
    fn default() -> Self {
        Self {
            active_theme: Default::default(),
            main_window_bounds: Default::default(),
            sidebar_width: Default::default(),
            main_page: Default::default(),
            page_path: Default::default(),
            quick_delete_mods: Default::default(),
            quick_delete_instance: Default::default(),
            instance_mods_sort_key: Default::default(),
            instance_mods_sort_enabled_first: Default::default(),
            instance_resourcepacks_sort_key: Default::default(),
            instance_resourcepacks_sort_enabled_first: Default::default(),
            instance_shaders_sort_key: Default::default(),
            instance_shaders_sort_enabled_first: Default::default(),
            content_install_latest: true,
            content_filter_version: Default::default(),
            modrinth_page_project_type: default_modrinth_project_type(),
            curseforge_page_class_id: default_curseforge_class_id(),
            hide_main_window_on_launch: false,
            quit_on_main_closed: false,
            use_os_titlebar: false,
            hide_server_addresses: false,
            hide_usernames: false,
            hide_skins: false,
            show_snapshots_in_create_instance: Default::default(),
            instances_view_mode: Default::default(),
            instance_subpage: Default::default(),
            collapse_capes_in_skins_page: false,
            skin_list_show_3d: true,
            modrinth_favorites: Default::default(),
            curseforge_favorites: Default::default(),
            modrinth_favorites_only: Default::default(),
            curseforge_favorites_only: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WindowBounds {
    #[default]
    Inherit,
    Windowed {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    },
    Maximized {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    },
    Fullscreen {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    },
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, strum::EnumIter)]
#[serde(rename_all = "lowercase")]
pub enum InstancesViewMode {
    #[default]
    Cards,
    List,
}

impl InstancesViewMode {
    pub fn name(self) -> SharedString {
        match self {
            InstancesViewMode::Cards => t::common::layout::cards().into(),
            InstancesViewMode::List => t::common::layout::list().into(),
        }
    }
}

impl InterfaceConfig {
    pub fn init(cx: &mut App, path: Arc<Path>) {
        cx.set_global(InterfaceConfigHolder {
            config: try_read_json(&path),
            write_task: None,
            path,
        });
    }

    pub fn get(cx: &App) -> &Self {
        &cx.global::<InterfaceConfigHolder>().config
    }

    pub fn force_save(cx: &mut App) {
        cx.global_mut::<InterfaceConfigHolder>().write_to_disk();
    }

    pub fn get_mut(cx: &mut App) -> &mut Self {
        if cx.global::<InterfaceConfigHolder>().write_task.is_none() {
            let task = cx.spawn(async |app| {
                app.background_executor().timer(Duration::from_secs(5)).await;
                _ = app.update_global::<InterfaceConfigHolder, _>(|holder, _| {
                    holder.write_to_disk();
                });
            });

            let holder = cx.global_mut::<InterfaceConfigHolder>();
            holder.write_task = Some(task);
            &mut holder.config
        } else {
            &mut cx.global_mut::<InterfaceConfigHolder>().config
        }
    }

    pub fn is_modrinth_favorite(&self, project_id: &str) -> bool {
        self.modrinth_favorites.iter().any(|f| f.project_id == project_id)
    }

    pub fn toggle_modrinth_favorite(&mut self, hit: &ModrinthHit) -> bool {
        if let Some(index) = self.modrinth_favorites.iter().position(|f| f.project_id == hit.project_id.as_ref()) {
            self.modrinth_favorites.remove(index);
            false
        } else {
            self.modrinth_favorites.push(ModrinthFavorite::from_hit(hit));
            true
        }
    }

    pub fn is_curseforge_favorite(&self, mod_id: u32) -> bool {
        self.curseforge_favorites.iter().any(|f| f.id == mod_id)
    }

    pub fn toggle_curseforge_favorite(&mut self, hit: &CurseforgeHit) -> bool {
        if let Some(index) = self.curseforge_favorites.iter().position(|f| f.id == hit.id) {
            self.curseforge_favorites.remove(index);
            false
        } else {
            self.curseforge_favorites.push(CurseforgeFavorite::from_hit(hit));
            true
        }
    }
}

impl InterfaceConfigHolder {
    fn write_to_disk(&mut self) {
        self.write_task = None;
        let Ok(bytes) = serde_json::to_vec(&self.config) else {
            return;
        };
        _ = write_safe(&self.path, &bytes);
    }
}

pub(crate) fn try_read_json<T: std::fmt::Debug + Default + for <'de> Deserialize<'de>>(path: &Path) -> T {
    let Ok(data) = std::fs::read(path) else {
        return T::default();
    };
    serde_json::from_slice(&data).unwrap_or_default()
}

pub(crate) fn write_safe(path: &Path, content: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut temp = path.to_path_buf();
    temp.add_extension(format!("{}", rand::thread_rng().next_u32()));
    temp.add_extension("new");

    let mut temp_file = std::fs::File::create(&temp)?;

    temp_file.write_all(content)?;
    temp_file.flush()?;
    temp_file.sync_all()?;

    drop(temp_file);

    if let Err(err) = std::fs::rename(&temp, path) {
        _ = std::fs::remove_file(&temp);
        return Err(err);
    }

    Ok(())
}
