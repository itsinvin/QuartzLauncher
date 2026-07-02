use std::sync::Arc;

use bridge::instance::{ContentFolder, ContentType, InstanceContentSummary, UNKNOWN_CONTENT_SUMMARY};
use gpui::SharedString;
use rustc_hash::FxHashMap;
use schema::{content::ContentSource, loader::Loader};

#[derive(Debug, Clone, Default)]
pub struct ContentConflictReport {
    pub messages: Vec<SharedString>,
    pub per_item: FxHashMap<u64, SharedString>,
}

pub fn detect_conflicts(
    content: &[InstanceContentSummary],
    folder: ContentFolder,
    instance_loader: Loader,
) -> ContentConflictReport {
    let mut report = ContentConflictReport::default();

    detect_duplicate_mod_ids(content, folder, &mut report);
    detect_duplicate_projects(content, &mut report);
    detect_duplicate_hashes(content, &mut report);
    detect_duplicate_names(content, &mut report);

    if folder == ContentFolder::Mods {
        detect_loader_mismatches(content, instance_loader, &mut report);
        if instance_loader == Loader::Fabric {
            detect_mixin_conflicts(content, &mut report);
        }
    }

    report
}

fn format_filenames(filenames: &[Arc<str>]) -> String {
    filenames
        .iter()
        .map(|name| name.as_ref())
        .collect::<Vec<_>>()
        .join(", ")
}

fn add_conflict(
    report: &mut ContentConflictReport,
    summaries: &[&InstanceContentSummary],
    message: SharedString,
) {
    report.messages.push(message.clone());
    for summary in summaries {
        let entry = report
            .per_item
            .entry(summary.filename_hash)
            .or_insert_with(|| SharedString::default());
        if entry.is_empty() {
            *entry = message.clone();
        } else {
            *entry = format!("{entry}\n{message}").into();
        }
    }
}

fn detect_duplicate_mod_ids(
    content: &[InstanceContentSummary],
    folder: ContentFolder,
    report: &mut ContentConflictReport,
) {
    if folder != ContentFolder::Mods {
        return;
    }

    let mut by_mod_id: FxHashMap<&str, Vec<&InstanceContentSummary>> = FxHashMap::default();

    for summary in content {
        let Some(mod_id) = summary.content_summary.id.as_deref() else {
            continue;
        };
        if mod_id.is_empty() {
            continue;
        }
        by_mod_id.entry(mod_id).or_default().push(summary);
    }

    for (mod_id, summaries) in by_mod_id {
        if summaries.len() < 2 {
            continue;
        }

        let enabled: Vec<_> = summaries.iter().copied().filter(|s| s.enabled).collect();
        if enabled.len() < 2 {
            continue;
        }

        let filenames: Vec<_> = enabled.iter().map(|s| Arc::clone(&s.filename)).collect();
        add_conflict(
            report,
            &enabled,
            t::instance::content::conflicts::duplicate_mod_id(mod_id, &format_filenames(&filenames)).into(),
        );
    }
}

fn detect_duplicate_projects(content: &[InstanceContentSummary], report: &mut ContentConflictReport) {
    let mut modrinth: FxHashMap<&str, Vec<&InstanceContentSummary>> = FxHashMap::default();
    let mut curseforge: FxHashMap<u32, Vec<&InstanceContentSummary>> = FxHashMap::default();

    for summary in content {
        match &summary.content_source {
            ContentSource::ModrinthProject { project_id } => {
                modrinth.entry(project_id.as_ref()).or_default().push(summary);
            },
            ContentSource::CurseforgeProject { project_id } => {
                curseforge.entry(*project_id).or_default().push(summary);
            },
            _ => {},
        }
    }

    for (project_id, summaries) in modrinth {
        if summaries.len() < 2 {
            continue;
        }

        let enabled: Vec<_> = summaries.iter().copied().filter(|s| s.enabled).collect();
        if enabled.len() < 2 {
            continue;
        }

        let filenames: Vec<_> = enabled.iter().map(|s| Arc::clone(&s.filename)).collect();
        add_conflict(
            report,
            &enabled,
            t::instance::content::conflicts::duplicate_modrinth_project(
                project_id,
                &format_filenames(&filenames),
            )
            .into(),
        );
    }

    for (project_id, summaries) in curseforge {
        if summaries.len() < 2 {
            continue;
        }

        let enabled: Vec<_> = summaries.iter().copied().filter(|s| s.enabled).collect();
        if enabled.len() < 2 {
            continue;
        }

        let filenames: Vec<_> = enabled.iter().map(|s| Arc::clone(&s.filename)).collect();
        add_conflict(
            report,
            &enabled,
            t::instance::content::conflicts::duplicate_curseforge_project(
                project_id,
                &format_filenames(&filenames),
            )
            .into(),
        );
    }
}

fn detect_duplicate_hashes(content: &[InstanceContentSummary], report: &mut ContentConflictReport) {
    let mut by_hash: FxHashMap<[u8; 20], Vec<&InstanceContentSummary>> = FxHashMap::default();

    for summary in content {
        if !summary.enabled {
            continue;
        }
        if summary.content_summary.hash == [0_u8; 20] {
            continue;
        }
        if Arc::ptr_eq(&summary.content_summary, &*UNKNOWN_CONTENT_SUMMARY) {
            continue;
        }
        by_hash
            .entry(summary.content_summary.hash)
            .or_default()
            .push(summary);
    }

    for summaries in by_hash.values() {
        if summaries.len() < 2 {
            continue;
        }

        let filenames: Vec<_> = summaries.iter().map(|s| Arc::clone(&s.filename)).collect();
        add_conflict(
            report,
            summaries,
            t::instance::content::conflicts::duplicate_file(&format_filenames(&filenames)).into(),
        );
    }
}

fn detect_duplicate_names(content: &[InstanceContentSummary], report: &mut ContentConflictReport) {
    let mut by_name: FxHashMap<&str, Vec<&InstanceContentSummary>> = FxHashMap::default();

    for summary in content {
        if !summary.enabled {
            continue;
        }
        let Some(name) = summary.content_summary.name.as_deref() else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        by_name.entry(name).or_default().push(summary);
    }

    for (name, summaries) in by_name {
        if summaries.len() < 2 {
            continue;
        }

        let distinct_files: FxHashMap<_, _> = summaries
            .iter()
            .map(|summary| (summary.filename.as_ref(), *summary))
            .collect();
        if distinct_files.len() < 2 {
            continue;
        }

        let filenames: Vec<_> = summaries.iter().map(|s| Arc::clone(&s.filename)).collect();
        add_conflict(
            report,
            &summaries,
            t::instance::content::conflicts::duplicate_name(name, &format_filenames(&filenames)).into(),
        );
    }
}

fn detect_loader_mismatches(
    content: &[InstanceContentSummary],
    instance_loader: Loader,
    report: &mut ContentConflictReport,
) {
    for summary in content {
        if !summary.enabled {
            continue;
        }

        let compatible = loader_compatible(instance_loader, &summary.content_summary.extra);
        if compatible {
            continue;
        }

        let loader_name = expected_loader_name(&summary.content_summary.extra);
        let message = t::instance::content::conflicts::loader_mismatch(
            summary.filename.as_ref(),
            loader_name,
            instance_loader.pretty_name(),
        );
        add_conflict(report, &[summary], message.into());
    }
}

fn loader_compatible(instance_loader: Loader, content_type: &ContentType) -> bool {
    match content_type {
        ContentType::Unknown | ContentType::ResourcePack | ContentType::ShaderPack => true,
        ContentType::ModrinthModpack { .. } | ContentType::CurseforgeModpack { .. } => true,
        ContentType::Fabric | ContentType::JavaModule => instance_loader == Loader::Fabric,
        ContentType::Forge | ContentType::LegacyForge => {
            instance_loader == Loader::Forge || instance_loader == Loader::NeoForge
        },
        ContentType::NeoForge => instance_loader == Loader::NeoForge,
    }
}

fn expected_loader_name(content_type: &ContentType) -> &'static str {
    match content_type {
        ContentType::Fabric | ContentType::JavaModule => Loader::Fabric.pretty_name(),
        ContentType::Forge | ContentType::LegacyForge => Loader::Forge.pretty_name(),
        ContentType::NeoForge => Loader::NeoForge.pretty_name(),
        _ => "Unknown",
    }
}

fn detect_mixin_conflicts(content: &[InstanceContentSummary], report: &mut ContentConflictReport) {
    let mut by_target: FxHashMap<&str, Vec<&InstanceContentSummary>> = FxHashMap::default();

    for summary in content {
        if !summary.enabled {
            continue;
        }
        if summary.content_summary.mixin_targets.is_empty() {
            continue;
        }

        for target in summary.content_summary.mixin_targets.iter() {
            by_target.entry(target.as_ref()).or_default().push(summary);
        }
    }

    for (target, summaries) in by_target {
        let unique: FxHashMap<_, _> = summaries
            .iter()
            .map(|summary| (summary.filename_hash, *summary))
            .collect();
        if unique.len() < 2 {
            continue;
        }

        let involved: Vec<_> = unique.values().copied().collect();
        let filenames: Vec<_> = involved.iter().map(|summary| Arc::clone(&summary.filename)).collect();
        add_conflict(
            report,
            &involved,
            t::instance::content::conflicts::mixin_target(target, &format_filenames(&filenames)).into(),
        );
    }
}
