use std::{path::{Path, PathBuf}, sync::Arc};

use bridge::{
    instance::{ContentFolder, ContentSummary, ContentType, ContentUpdateContext, ContentUpdateStatus, InstanceContentSummary},
    message::{EmbeddedOrRaw, MessageToFrontend},
    modal_action::{ModalAction, ProgressTracker, ProgressTrackerFinishType},
};
use schema::{instance::InstanceConfiguration, loader::Loader};
use ustr::Ustr;

use crate::{BackendState, FolderChanges};

enum ModpackFolderKind {
    Modrinth,
    Curseforge,
    Extracted {
        dot_minecraft_source: PathBuf,
        configuration: Option<InstanceConfiguration>,
    },
}

impl BackendState {
    pub async fn import_modpack_from_folder(self: &Arc<Self>, folder: PathBuf, modal_action: ModalAction) {
        let folder = match folder.canonicalize() {
            Ok(folder) => folder,
            Err(err) => {
                modal_action.set_error_message(format!("Unable to read folder: {err}").into());
                modal_action.set_finished();
                return;
            },
        };

        if !folder.is_dir() {
            modal_action.set_error_message("Selected path is not a folder".into());
            modal_action.set_finished();
            return;
        }

        match classify_modpack_folder(&folder) {
            Ok(ModpackFolderKind::Modrinth) => {
                self.import_indexed_modpack_folder(&folder, true, modal_action).await;
            },
            Ok(ModpackFolderKind::Curseforge) => {
                self.import_indexed_modpack_folder(&folder, false, modal_action).await;
            },
            Ok(ModpackFolderKind::Extracted { dot_minecraft_source, configuration }) => {
                self.import_extracted_modpack_folder(&folder, dot_minecraft_source, configuration, modal_action).await;
            },
            Err(message) => {
                modal_action.set_error_message(message.into());
                modal_action.set_finished();
            },
        }
    }

    async fn import_indexed_modpack_folder(
        self: &Arc<Self>,
        folder: &Path,
        modrinth: bool,
        modal_action: ModalAction,
    ) {
        let content_summary = if modrinth {
            self.mod_metadata_manager.load_modrinth_modpack_from_folder(folder)
        } else {
            self.mod_metadata_manager.load_curseforge_modpack_from_folder(folder)
        };

        let Some(content_summary) = content_summary else {
            modal_action.set_error_message("Unable to read modpack metadata from folder".into());
            modal_action.set_finished();
            return;
        };

        let Some(name) = content_summary.name.clone() else {
            modal_action.set_error_message("Unable to determine modpack name".into());
            modal_action.set_finished();
            return;
        };

        let Some((loader, minecraft_version)) = loader_and_version_from_summary(&content_summary) else {
            modal_action.set_error_message("Unable to determine Minecraft version or mod loader from modpack".into());
            modal_action.set_finished();
            return;
        };

        modal_action.append_log(format!("Creating instance \"{name}\"…"), &self.send);

        let icon = content_summary.png_icon.clone().map(EmbeddedOrRaw::Raw);
        let Some(instance_dir) = self.create_instance_sanitized(&name, &minecraft_version, loader, icon).await else {
            modal_action.set_error_message("Unable to create instance".into());
            modal_action.set_finished();
            return;
        };

        let dot_minecraft_dir = instance_dir.join(".minecraft");
        let instance_content_summary = synthetic_instance_content_summary(content_summary.clone(), folder);

        modal_action.append_log("Downloading and installing modpack files…", &self.send);
        self.download_modpack_children(&instance_content_summary, loader, minecraft_version, &modal_action, false).await;

        let affected = self.apply_modpack_content_summary_to_instance(
            &content_summary,
            &dot_minecraft_dir,
            &modal_action,
            None,
            None,
        );

        self.load_instance_from_path(&instance_dir, true, true);

        if let Some(instance) = self.instance_state.write().instances.iter_mut().find(|i| i.root_path.as_ref() == instance_dir.as_path()) {
            instance.mark_content_dirty(self, ContentFolder::Mods, FolderChanges::all_dirty(), true);
            if affected.resource_packs {
                instance.mark_content_dirty(self, ContentFolder::ResourcePacks, FolderChanges::all_dirty(), true);
            }
            if affected.shaders {
                instance.mark_content_dirty(self, ContentFolder::Shaders, FolderChanges::all_dirty(), true);
            }
        }

        modal_action.append_log("Modpack folder import complete.", &self.send);
        modal_action.set_finished();
        self.send.send(MessageToFrontend::Refresh);
    }

    async fn import_extracted_modpack_folder(
        self: &Arc<Self>,
        folder: &Path,
        dot_minecraft_source: PathBuf,
        configuration: Option<InstanceConfiguration>,
        modal_action: ModalAction,
    ) {
        let default_name = folder
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Imported Modpack".into());

        let (loader, minecraft_version, name) = if let Some(configuration) = configuration {
            (
                configuration.loader,
                configuration.minecraft_version.to_string(),
                default_name,
            )
        } else if let Some((loader, version)) = detect_loader_and_version(&dot_minecraft_source) {
            (loader, version, default_name)
        } else {
            modal_action.set_error_message(
                "Could not detect Minecraft version or mod loader. \
                 Include modrinth.index.json / manifest.json, or MultiMC instance.cfg + mmc-pack.json, \
                 or loader jars in mods/."
                    .into(),
            );
            modal_action.set_finished();
            return;
        };

        modal_action.append_log(format!("Creating instance \"{name}\"…"), &self.send);

        let icon_path = folder.join("icon.png");
        let icon = std::fs::read(&icon_path).ok().map(|bytes| EmbeddedOrRaw::Raw(bytes.into()));

        let Some(instance_dir) = self.create_instance_sanitized(&name, &minecraft_version, loader, icon).await else {
            modal_action.set_error_message("Unable to create instance".into());
            modal_action.set_finished();
            return;
        };

        let dot_minecraft_dir = instance_dir.join(".minecraft");
        if let Err(err) = std::fs::create_dir_all(&dot_minecraft_dir) {
            modal_action.set_error_message(format!("Unable to create .minecraft folder: {err}").into());
            modal_action.set_finished();
            return;
        }

        modal_action.append_log("Copying modpack files into instance…", &self.send);
        let tracker = ProgressTracker::new("Copying modpack folder".into(), self.send.clone());
        modal_action.trackers.push(tracker.clone());

        let copy_result = crate::copy_content_recursive(
            &dot_minecraft_source,
            &dot_minecraft_dir,
            false,
            &|copied, total| {
                if total > 0 {
                    tracker.set_total(total as usize);
                    tracker.set_count(copied as usize);
                    tracker.notify();
                }
            },
        );

        if let Err(err) = copy_result {
            modal_action.set_error_message(format!("Failed to copy modpack files: {err}").into());
            modal_action.set_finished();
            return;
        }

        tracker.set_finished(ProgressTrackerFinishType::Normal);
        tracker.notify();

        self.load_instance_from_path(&instance_dir, true, true);

        if let Some(instance) = self.instance_state.write().instances.iter_mut().find(|i| i.root_path.as_ref() == instance_dir.as_path()) {
            instance.mark_content_dirty(self, ContentFolder::Mods, FolderChanges::all_dirty(), true);
            instance.mark_content_dirty(self, ContentFolder::ResourcePacks, FolderChanges::all_dirty(), true);
            instance.mark_content_dirty(self, ContentFolder::Shaders, FolderChanges::all_dirty(), true);
        }

        modal_action.append_log("Modpack folder import complete.", &self.send);
        modal_action.set_finished();
        self.send.send(MessageToFrontend::Refresh);
    }
}

fn classify_modpack_folder(folder: &Path) -> Result<ModpackFolderKind, &'static str> {
    if folder.join("modrinth.index.json").is_file() {
        return Ok(ModpackFolderKind::Modrinth);
    }

    if folder.join("manifest.json").is_file() {
        if let Ok(bytes) = std::fs::read(folder.join("manifest.json")) {
            if serde_json::from_slice::<schema::curseforge::CurseforgeModpackManifestJson>(&bytes).is_ok() {
                return Ok(ModpackFolderKind::Curseforge);
            }
        }
    }

    if let Some(configuration) = crate::launcher_import::try_load_from_other_launcher_formats(folder) {
        let dot_minecraft_source = resolve_dot_minecraft_source(folder)?;
        return Ok(ModpackFolderKind::Extracted {
            dot_minecraft_source,
            configuration: Some(configuration),
        });
    }

    let dot_minecraft_source = resolve_dot_minecraft_source(folder)?;
    Ok(ModpackFolderKind::Extracted {
        dot_minecraft_source,
        configuration: None,
    })
}

fn resolve_dot_minecraft_source(folder: &Path) -> Result<PathBuf, &'static str> {
    let dot_minecraft = folder.join(".minecraft");
    if dot_minecraft.is_dir() && dot_minecraft.join("mods").is_dir() {
        return Ok(dot_minecraft);
    }

    let minecraft = folder.join("minecraft");
    if minecraft.is_dir() && minecraft.join("mods").is_dir() {
        return Ok(minecraft);
    }

    if folder.join("mods").is_dir() {
        return Ok(folder.to_path_buf());
    }

    Err("Folder must contain mods/ (or .minecraft/mods/) or modrinth.index.json / manifest.json")
}

fn loader_and_version_from_summary(content_summary: &ContentSummary) -> Option<(Loader, Ustr)> {
    match &content_summary.extra {
        ContentType::ModrinthModpack { dependencies, .. } => {
            let mut minecraft_version = None;
            let mut loader = Loader::Vanilla;
            for (key, value) in dependencies {
                match &**key {
                    "forge" => loader = Loader::Forge,
                    "neoforge" => loader = Loader::NeoForge,
                    "fabric-loader" => loader = Loader::Fabric,
                    "minecraft" => minecraft_version = Some(value.clone()),
                    _ => {},
                }
            }
            Some((loader, minecraft_version?.into()))
        },
        ContentType::CurseforgeModpack { minecraft, .. } => {
            let loader = minecraft.get_loader()?;
            let version = minecraft.version.clone()?;
            Some((loader, version.into()))
        },
        _ => None,
    }
}

fn detect_loader_and_version(dot_minecraft: &Path) -> Option<(Loader, String)> {
    let mods_dir = dot_minecraft.join("mods");
    let Ok(read_dir) = std::fs::read_dir(&mods_dir) else {
        return None;
    };

    let mut loader = None;
    let mut version = None;

    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let file_name = path.file_name()?.to_string_lossy().to_ascii_lowercase();
        if loader.is_none() {
            loader = detect_loader_from_filename(&file_name);
        }

        if version.is_none() {
            version = detect_version_from_mod_jar(&path);
        }

        if loader.is_some() && version.is_some() {
            break;
        }
    }

    Some((loader?, version?))
}

fn detect_loader_from_filename(file_name: &str) -> Option<Loader> {
    if file_name.contains("fabric-loader") || file_name.contains("fabricloader") {
        Some(Loader::Fabric)
    } else if file_name.contains("neoforge") {
        Some(Loader::NeoForge)
    } else if file_name.contains("forge") {
        Some(Loader::Forge)
    } else {
        None
    }
}

fn detect_version_from_mod_jar(path: &Path) -> Option<String> {
    use std::io::Read;

    let file = std::fs::File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;

    if let Ok(mut fabric_mod) = archive.by_name("fabric.mod.json") {
        let mut contents = String::new();
        fabric_mod.read_to_string(&mut contents).ok()?;
        let json: serde_json::Value = serde_json::from_str(&contents).ok()?;
        if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
            if id.contains("fabricloader") || id.contains("fabric-loader") {
                return None;
            }
        }
    }

    if let Ok(mut mods_toml) = archive.by_name("META-INF/mods.toml") {
        let mut contents = String::new();
        mods_toml.read_to_string(&mut contents).ok()?;
        for line in contents.lines() {
            let line = line.trim();
            if let Some(value) = line.strip_prefix("minecraft = \"") {
                let value = value.trim_end_matches('"');
                if !value.is_empty() {
                    return Some(value.split(',').next()?.trim().to_string());
                }
            }
        }
    }

    None
}

fn synthetic_instance_content_summary(content_summary: Arc<ContentSummary>, folder: &Path) -> InstanceContentSummary {
    use bridge::instance::{ContentUpdateContext, InstanceContentID};
    use schema::{auxiliary::AuxDisabledChildren, content::ContentSource};

    let (loader, version) = loader_and_version_from_summary(&content_summary).unwrap_or((Loader::Vanilla, "unknown".into()));

    InstanceContentSummary {
        content_summary,
        id: InstanceContentID::dangling(),
        filename: folder
            .file_name()
            .map(|name| name.to_string_lossy().into())
            .unwrap_or_else(|| "modpack-folder".into()),
        lowercase_search_keys: Arc::from([]),
        filename_hash: 0,
        modified_unix_ms: 0,
        path: folder.into(),
        can_toggle: false,
        enabled: true,
        content_source: ContentSource::Manual,
        update: ContentUpdateContext::new(ContentUpdateStatus::Unknown, loader, version.as_str()),
        disabled_children: Arc::new(AuxDisabledChildren::default()),
    }
}
