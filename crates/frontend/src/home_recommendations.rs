use std::sync::Arc;

use bridge::instance::ContentFolder;
use rustc_hash::FxHashSet;
use schema::{
    loader::Loader,
    modrinth::{ModrinthHit, ModrinthSearchIndex, ModrinthSearchRequest},
};

use crate::entity::instance::InstanceEntry;

pub struct RecommendationContext {
    pub total_mods: usize,
    pub installed_mod_names: FxHashSet<Arc<str>>,
    pub favorite_project_ids: FxHashSet<Arc<str>>,
}

impl RecommendationContext {
    pub fn from_instances(instances: &[InstanceEntry], favorite_ids: &[String], cx: &gpui::App) -> Self {
        let mut installed_mod_names = FxHashSet::default();
        for instance in instances {
            for summary in instance.content[ContentFolder::Mods].read(cx).iter() {
                installed_mod_names.insert(summary.filename.clone());
                if let Some(name) = &summary.content_summary.name {
                    installed_mod_names.insert(name.clone());
                }
            }
        }

        Self {
            total_mods: installed_mod_names.len(),
            installed_mod_names,
            favorite_project_ids: favorite_ids.iter().map(|id| Arc::from(id.as_str())).collect(),
        }
    }
}

pub fn build_search_request(instance: &InstanceEntry, ctx: &RecommendationContext) -> ModrinthSearchRequest {
    let mc_version = instance.configuration.minecraft_version.as_str();
    let loader = instance.configuration.loader;
    let loader_id = loader.as_modrinth_loader().id();

    let query = recommendation_query(loader, ctx.total_mods);

    let facets = format!(
        "[[\"project_type=mod\"],[\"versions={mc_version}\"],[\"categories:{loader_id}\"]]"
    );

    ModrinthSearchRequest {
        query,
        facets: Some(facets.into()),
        index: ModrinthSearchIndex::Downloads,
        offset: 0,
        limit: 20,
    }
}

fn recommendation_query(loader: Loader, total_mods: usize) -> Option<Arc<str>> {
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

pub fn rank_recommendations(hits: &[ModrinthHit], ctx: &RecommendationContext, limit: usize) -> Vec<ModrinthHit> {
    let mut scored: Vec<(i32, ModrinthHit)> = hits
        .iter()
        .filter(|hit| !ctx.favorite_project_ids.contains(hit.project_id.as_ref()))
        .filter(|hit| !is_already_installed(hit, &ctx.installed_mod_names))
        .map(|hit| (score_hit(hit, ctx), hit.clone()))
        .collect();

    scored.sort_by(|(a, hit_a), (b, hit_b)| {
        b.cmp(a).then_with(|| hit_b.downloads.cmp(&hit_a.downloads))
    });

    scored.into_iter().take(limit).map(|(_, hit)| hit).collect()
}

fn score_hit(hit: &ModrinthHit, ctx: &RecommendationContext) -> i32 {
    let mut score = 0i32;

    let downloads = hit.downloads.min(i32::MAX as u64) as i32;
    score += (downloads / 100_000).min(40);

    if ctx.total_mods > 30 {
        if hit
            .display_categories
            .as_ref()
            .is_some_and(|cats| cats.iter().any(|c| c.as_str() == "optimization" || c.as_str() == "performance"))
        {
            score += 25;
        }
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
