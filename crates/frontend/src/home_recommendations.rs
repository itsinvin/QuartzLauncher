use std::sync::Arc;

use bridge::instance::{ContentFolder, ContentType as BridgeContentType, InstanceContentSummary};
use rustc_hash::FxHashSet;
use schema::{
    loader::Loader,
    modrinth::{ModrinthHit, ModrinthProjectType, ModrinthSearchIndex, ModrinthSearchRequest},
};

use crate::entity::instance::InstanceEntry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecommendedContentKind {
    Mod,
    Modpack,
    ResourcePack,
    Shader,
}

impl RecommendedContentKind {
    pub fn modrinth_project_type(self) -> ModrinthProjectType {
        match self {
            Self::Mod => ModrinthProjectType::Mod,
            Self::Modpack => ModrinthProjectType::Modpack,
            Self::ResourcePack => ModrinthProjectType::Resourcepack,
            Self::Shader => ModrinthProjectType::Shader,
        }
    }

    pub fn content_folder(self) -> Option<ContentFolder> {
        match self {
            Self::Mod => Some(ContentFolder::Mods),
            Self::ResourcePack => Some(ContentFolder::ResourcePacks),
            Self::Shader => Some(ContentFolder::Shaders),
            Self::Modpack => None,
        }
    }
}

pub struct RecommendationContext {
    pub total_items: usize,
    pub installed_names: FxHashSet<Arc<str>>,
    pub exclude_project_ids: FxHashSet<Arc<str>>,
}

impl RecommendationContext {
    pub fn for_instance_folder(
        instance: &InstanceEntry,
        folder: ContentFolder,
        exclude_project_ids: &[String],
        cx: &gpui::App,
    ) -> Self {
        Self::from_installed_content(instance.content[folder].read(cx), exclude_project_ids)
    }

    pub fn from_installed_content(
        content: &[InstanceContentSummary],
        exclude_project_ids: &[String],
    ) -> Self {
        let mut installed_names = FxHashSet::default();

        for summary in content {
            installed_names.insert(summary.filename.clone());
            if let Some(name) = &summary.content_summary.name {
                installed_names.insert(name.clone());
            }
        }

        Self {
            total_items: installed_names.len(),
            installed_names,
            exclude_project_ids: exclude_project_ids.iter().map(|id| Arc::from(id.as_str())).collect(),
        }
    }

    pub fn for_modpacks(instances: &[InstanceEntry], exclude_project_ids: &[String], cx: &gpui::App) -> Self {
        let mut installed_names = FxHashSet::default();
        let mut total_mods = 0usize;

        for instance in instances {
            installed_names.insert(Arc::from(instance.name.as_ref()));

            for summary in instance.content[ContentFolder::Mods].read(cx).iter() {
                total_mods += 1;
                installed_names.insert(summary.filename.clone());
                if let Some(name) = &summary.content_summary.name {
                    installed_names.insert(name.clone());
                }

                if matches!(
                    summary.content_summary.extra,
                    BridgeContentType::ModrinthModpack { .. } | BridgeContentType::CurseforgeModpack { .. }
                ) {
                    if let Some(name) = &summary.content_summary.name {
                        installed_names.insert(name.clone());
                    }
                }
            }
        }

        Self {
            total_items: total_mods.max(instances.len()),
            installed_names,
            exclude_project_ids: exclude_project_ids.iter().map(|id| Arc::from(id.as_str())).collect(),
        }
    }
}

pub fn build_search_request(
    loader: Loader,
    minecraft_version: &str,
    kind: RecommendedContentKind,
    ctx: &RecommendationContext,
) -> ModrinthSearchRequest {
    let mc_version = minecraft_version;
    let loader_id = loader.as_modrinth_loader().id();
    let project_type = kind.modrinth_project_type().as_str();

    let query = match kind {
        RecommendedContentKind::Modpack => modpack_recommendation_query(loader, ctx.total_items),
        RecommendedContentKind::Mod => mod_recommendation_query(loader, ctx.total_items),
        RecommendedContentKind::ResourcePack => Some(Arc::from("aesthetic")),
        RecommendedContentKind::Shader => {
            if ctx.total_items > 0 {
                Some(Arc::from("performance"))
            } else {
                Some(Arc::from("realistic"))
            }
        }
    };

    let facets = if loader == Loader::Vanilla && kind != RecommendedContentKind::Modpack {
        format!("[[\"project_type={project_type}\"],[\"versions={mc_version}\"]]")
    } else {
        format!(
            "[[\"project_type={project_type}\"],[\"versions={mc_version}\"],[\"categories:{loader_id}\"]]"
        )
    };

    ModrinthSearchRequest {
        query,
        facets: Some(facets.into()),
        index: ModrinthSearchIndex::Downloads,
        offset: 0,
        limit: 20,
    }
}

fn mod_recommendation_query(loader: Loader, total_mods: usize) -> Option<Arc<str>> {
    match loader {
        Loader::Fabric => {
            if total_mods > 35 {
                Some(Arc::from("performance optimization"))
            } else if total_mods > 12 {
                Some(Arc::from("utility quality of life"))
            } else {
                Some(Arc::from("library api"))
            }
        }
        Loader::Forge | Loader::NeoForge => {
            if total_mods > 35 {
                Some(Arc::from("optimization"))
            } else {
                Some(Arc::from("tech"))
            }
        }
        _ => {
            if total_mods > 25 {
                Some(Arc::from("optimization"))
            } else {
                None
            }
        }
    }
}

fn modpack_recommendation_query(loader: Loader, signal: usize) -> Option<Arc<str>> {
    match loader {
        Loader::Fabric => {
            if signal > 80 {
                Some(Arc::from("kitchen sink"))
            } else if signal > 35 {
                Some(Arc::from("questing adventure"))
            } else if signal > 12 {
                Some(Arc::from("vanilla plus"))
            } else {
                Some(Arc::from("lightweight survival"))
            }
        }
        Loader::Forge | Loader::NeoForge => {
            if signal > 50 {
                Some(Arc::from("tech modpack"))
            } else {
                Some(Arc::from("adventure quests"))
            }
        }
        Loader::Vanilla => Some(Arc::from("vanilla")),
    }
}

pub fn rank_recommendations(
    hits: &[ModrinthHit],
    kind: RecommendedContentKind,
    ctx: &RecommendationContext,
    limit: usize,
) -> Vec<ModrinthHit> {
    let mut scored: Vec<(i32, ModrinthHit)> = hits
        .iter()
        .filter(|hit| hit.project_type == kind.modrinth_project_type())
        .filter(|hit| !ctx.exclude_project_ids.contains(hit.project_id.as_ref()))
        .filter(|hit| !is_already_installed(hit, &ctx.installed_names))
        .map(|hit| (score_hit(hit, kind, ctx), hit.clone()))
        .collect();

    scored.sort_by(|(a, hit_a), (b, hit_b)| {
        b.cmp(a).then_with(|| hit_b.downloads.cmp(&hit_a.downloads))
    });

    scored.into_iter().take(limit).map(|(_, hit)| hit).collect()
}

fn score_hit(hit: &ModrinthHit, kind: RecommendedContentKind, ctx: &RecommendationContext) -> i32 {
    let mut score = 0i32;

    let downloads = hit.downloads.min(i32::MAX as u64) as i32;
    score += (downloads / 100_000).min(40);

    match kind {
        RecommendedContentKind::Mod if ctx.total_items > 30 => {
            if hit.display_categories.as_ref().is_some_and(|cats| {
                cats.iter()
                    .any(|c| c.as_str() == "optimization" || c.as_str() == "performance")
            }) {
                score += 25;
            }
        }
        RecommendedContentKind::Modpack => {
            if hit
                .display_categories
                .as_ref()
                .is_some_and(|cats| cats.iter().any(|c| c.as_str() == "quests" || c.as_str() == "adventure"))
            {
                score += 15;
            }
            if ctx.total_items > 40 {
                if hit.title.as_ref().is_some_and(|t| {
                    let lower = t.to_ascii_lowercase();
                    lower.contains("kitchen") || lower.contains("atm") || lower.contains("all the mods")
                }) {
                    score += 20;
                }
            }
        }
        RecommendedContentKind::Shader => {
            if hit
                .display_categories
                .as_ref()
                .is_some_and(|cats| cats.iter().any(|c| c.as_str() == "performance"))
            {
                score += 20;
            }
        }
        RecommendedContentKind::ResourcePack => {
            if hit
                .display_categories
                .as_ref()
                .is_some_and(|cats| cats.iter().any(|c| c.as_str() == "128x" || c.as_str() == "256x"))
            {
                score += 10;
            }
        }
        _ => {}
    }

    if hit
        .display_categories
        .as_ref()
        .is_some_and(|cats| cats.iter().any(|c| c.as_str() == "library" || c.as_str() == "api"))
    {
        score += 8;
    }

    score
}

fn is_already_installed(hit: &ModrinthHit, installed: &FxHashSet<Arc<str>>) -> bool {
    let title = hit.title.as_deref().unwrap_or("").to_ascii_lowercase();
    if title.is_empty() {
        return false;
    }

    installed.iter().any(|name| {
        let lower = name.to_ascii_lowercase();
        lower.contains(&title) || title.contains(&lower)
    })
}
