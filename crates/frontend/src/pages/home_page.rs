use bridge::{
    handle::BackendHandle,
    instance::ContentFolder,
    message::{AccountSkinResult, MessageToBackend},
};
use bridge::meta::MetadataRequest;
use gpui::{prelude::FluentBuilder as _, *};
use gpui_component::{
    ActiveTheme as _, Disableable, Icon, Sizable, StyledExt,
    button::{Button, ButtonVariants},
    h_flex, skeleton::Skeleton, v_flex,
};

use once_cell::sync::Lazy;
use schema::{
    minecraft_profile::SkinVariant,
    modrinth::ModrinthHit,
    unique_bytes::UniqueBytes,
};

use crate::{
    component::{
        animation::{self, format_last_played, format_playtime},
        instance_list::{InstanceList, instance_is_active},
        player_model_widget::PlayerModelWidget,
        quartz_logo::QuartzLogo,
        responsive_grid::ResponsiveGrid,
    },
    entity::{
        DataEntities,
        account::AccountExt,
        instance::{InstanceAddedEvent, InstanceEntry, InstanceModifiedEvent, InstanceRemovedEvent},
        metadata::{AsMetadataResult, FrontendMetadata, FrontendMetadataResult},
    },
    home_recommendations::{RecommendationContext, build_search_request, rank_recommendations},
    icon::QuartzIcon,
    interface_config::{skin_fingerprint, CurseforgeFavorite, InterfaceConfig, ModrinthFavorite},
    pages::page::Page,
    png_render_cache,
    root,
    skin_thumbnail_cache::SkinThumbnailCache,
    ui::PageType,
    MINECRAFT_FONT,
};

static DEFAULT_SKIN: Lazy<UniqueBytes> = Lazy::new(|| {
    UniqueBytes::new(include_bytes!("../../../../assets/images/default_skin.png"))
});

pub struct HomePage {
    data: DataEntities,
    backend_handle: BackendHandle,
    player_model: Entity<PlayerModelWidget>,
    skin_thumbnail_cache: Entity<SkinThumbnailCache>,
    account_skin: UniqueBytes,
    account_skin_variant: SkinVariant,
    request_account_skin: Option<Task<()>>,
    refresh_generation: u64,
    recommended_hits: Vec<ModrinthHit>,
    recommendations_loading: bool,
    recommendations_error: Option<SharedString>,
    recommendations_generation: u64,
    _recommendations_subscription: Option<Subscription>,
    _instance_added_subscription: Subscription,
    _instance_modified_subscription: Subscription,
    _instance_removed_subscription: Subscription,
}

impl HomePage {
    pub fn new(data: &DataEntities, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let player_model = cx.new(|cx| PlayerModelWidget::new_preview(cx, DEFAULT_SKIN.clone()));
        let skin_thumbnail_cache = SkinThumbnailCache::new(cx);
        let instances = data.instances.clone();

        let _instance_added_subscription =
            cx.subscribe::<_, InstanceAddedEvent>(&instances, |_, _, _, cx| cx.notify());
        let _instance_modified_subscription =
            cx.subscribe::<_, InstanceModifiedEvent>(&instances, |_, _, _, cx| cx.notify());
        let _instance_removed_subscription =
            cx.subscribe::<_, InstanceRemovedEvent>(&instances, |_, _, _, cx| cx.notify());

        let mut page = Self {
            data: data.clone(),
            backend_handle: data.backend_handle.clone(),
            player_model,
            skin_thumbnail_cache,
            account_skin: DEFAULT_SKIN.clone(),
            account_skin_variant: SkinVariant::Classic,
            request_account_skin: None,
            refresh_generation: 0,
            recommended_hits: Vec::new(),
            recommendations_loading: false,
            recommendations_error: None,
            recommendations_generation: 0,
            _recommendations_subscription: None,
            _instance_added_subscription,
            _instance_modified_subscription,
            _instance_removed_subscription,
        };
        page.load_account_skin(cx);
        page
    }

    fn load_account_skin(&mut self, cx: &mut Context<Self>) {
        if self.request_account_skin.is_some() {
            return;
        }

        let Some(uuid) = self.data.accounts.read(cx).selected_account_uuid else {
            return;
        };

        let (send, recv) = tokio::sync::oneshot::channel();
        self.backend_handle.send(MessageToBackend::GetAccountSkin {
            account: uuid,
            result: send,
        });

        self.request_account_skin = Some(cx.spawn(async move |page, cx| {
            let Ok(result) = recv.await else {
                return;
            };
            let _ = page.update(cx, |page, cx| {
                if let AccountSkinResult::Success { skin, variant } = result {
                    if let Some(skin) = skin {
                        page.account_skin = skin.clone();
                        page.account_skin_variant = variant;
                        page.player_model.update(cx, |widget, cx| {
                            widget.set_skin(cx, skin, variant);
                        });
                    }
                }
                page.request_account_skin = None;
                cx.notify();
            });
        }));
    }

    fn sorted_instances(&self, cx: &App) -> Vec<InstanceEntry> {
        let mut items = self
            .data
            .instances
            .read(cx)
            .entries
            .values()
            .map(|entry| entry.read(cx).clone())
            .collect::<Vec<_>>();
        InstanceList::sort_by_last_played(&mut items);
        items
    }

    fn aggregate_stats(&self, instances: &[InstanceEntry], cx: &App) -> HomeStats {
        let mut total_playtime_secs = 0u64;
        let mut total_mods = 0usize;

        for instance in instances {
            total_playtime_secs += instance.playtime.total_secs;
            total_mods += instance.content[ContentFolder::Mods].read(cx).len();
        }

        HomeStats {
            instance_count: instances.len(),
            total_playtime_secs,
            total_mods,
        }
    }

    fn ensure_recommendations_loaded(&mut self, instances: &[InstanceEntry], cx: &mut Context<Self>) {
        if self.recommendations_loading || self._recommendations_subscription.is_some() {
            return;
        }

        let Some(instance) = instances.first() else {
            self.recommended_hits.clear();
            return;
        };

        let favorite_ids = InterfaceConfig::get(cx)
            .modrinth_favorites
            .iter()
            .map(|f| f.project_id.clone())
            .collect::<Vec<_>>();
        let ctx = RecommendationContext::from_instances(instances, &favorite_ids, cx);
        let request = build_search_request(instance, &ctx);
        let generation = self.recommendations_generation;

        self.recommendations_loading = true;
        self.recommendations_error = None;

        let data = FrontendMetadata::request(&self.data.metadata, MetadataRequest::ModrinthSearch(request), cx);
        let subscription = cx.observe(&data, move |page, data, cx| {
            let result: FrontendMetadataResult<schema::modrinth::ModrinthSearchResult> = data.read(cx).result();
            match result {
                FrontendMetadataResult::Loading => {}
                FrontendMetadataResult::Loaded(search_result) => {
                    if page.recommendations_generation != generation {
                        return;
                    }
                    let favorite_ids = InterfaceConfig::get(cx)
                        .modrinth_favorites
                        .iter()
                        .map(|f| f.project_id.clone())
                        .collect::<Vec<_>>();
                    let instances = page.sorted_instances(cx);
                    let ctx = RecommendationContext::from_instances(&instances, &favorite_ids, cx);
                    page.recommended_hits = rank_recommendations(&search_result.hits, &ctx, 6);
                    page.recommendations_loading = false;
                    page._recommendations_subscription = None;
                    cx.notify();
                }
                FrontendMetadataResult::Error(error) => {
                    if page.recommendations_generation != generation {
                        return;
                    }
                    page.recommendations_error = Some(error);
                    page.recommendations_loading = false;
                    page._recommendations_subscription = None;
                    cx.notify();
                }
            }
        });

        self._recommendations_subscription = Some(subscription);

        let result: FrontendMetadataResult<schema::modrinth::ModrinthSearchResult> = data.read(cx).result();
        if let FrontendMetadataResult::Loaded(search_result) = result {
            self.recommended_hits = rank_recommendations(&search_result.hits, &ctx, 6);
            self.recommendations_loading = false;
            self._recommendations_subscription = None;
        } else if let FrontendMetadataResult::Error(error) = result {
            self.recommendations_error = Some(error);
            self.recommendations_loading = false;
            self._recommendations_subscription = None;
        }
    }

    fn resolve_recent_skins(&self, cx: &mut Context<Self>) -> Vec<(UniqueBytes, SkinVariant)> {
        let mut skins = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let recent_entries = InterfaceConfig::get(cx).recent_skins.clone();

        let account_fp = skin_fingerprint(&self.account_skin);
        seen.insert(account_fp);
        skins.push((self.account_skin.clone(), self.account_skin_variant));

        let library_skins = self
            .data
            .use_skin_library(cx)
            .map(|library| library.skins.clone());

        if let Some(library_skins) = library_skins {
            for entry in recent_entries.iter() {
                if skins.len() >= 8 {
                    break;
                }
                if seen.contains(&entry.fingerprint) {
                    continue;
                }
                if let Some(skin) = library_skins
                    .iter()
                    .find(|skin| skin_fingerprint(skin) == entry.fingerprint)
                {
                    seen.insert(entry.fingerprint.clone());
                    let variant = crate::skin_renderer::determine_skin_variant(skin).unwrap_or(SkinVariant::Classic);
                    skins.push((skin.clone(), variant));
                }
            }
        }

        skins
    }
}

struct HomeStats {
    instance_count: usize,
    total_playtime_secs: u64,
    total_mods: usize,
}

impl Page for HomePage {
    fn controls(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Button::new("home_refresh")
            .compact()
            .icon(QuartzIcon::RefreshCcw)
            .label(t::home::refresh())
            .on_click(cx.listener(|this, _, _window, cx| {
                this.refresh_generation = this.refresh_generation.wrapping_add(1);
                this.recommendations_generation = this.recommendations_generation.wrapping_add(1);
                this.recommended_hits.clear();
                this._recommendations_subscription = None;
                this.recommendations_loading = false;
                this.load_account_skin(cx);
                cx.notify();
            }))
    }

    fn scrollable(&self, _cx: &App) -> bool {
        true
    }
}

impl Render for HomePage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let instances = self.sorted_instances(cx);

        if instances.is_empty() {
            let logo_scale = animation::animated_logo_scale(window, cx);
            let theme = cx.theme();
            let logo_size = px(72.0 * logo_scale);
            return v_flex()
                .size_full()
                .p_8()
                .gap_6()
                .justify_center()
                .items_center()
                .child(
                    v_flex()
                        .gap_4()
                        .items_center()
                        .child(
                            div()
                                .p_4()
                                .rounded(theme.radius_lg)
                                .border_1()
                                .border_color(theme.sidebar_primary.opacity(0.35))
                                .bg(theme.muted)
                                .child(QuartzLogo::new(logo_size)),
                        )
                        .child(
                            div()
                                .text_2xl()
                                .font_family(SharedString::new_static(MINECRAFT_FONT))
                                .text_color(theme.sidebar_primary)
                                .child(t::home::welcome_title()),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .max_w_96()
                                .text_center()
                                .child(t::home::welcome_subtitle()),
                        )
                        .child(
                            Button::new("home_create")
                                .success()
                                .icon(QuartzIcon::Plus)
                                .label(t::instance::create())
                                .on_click(cx.listener(|this, _, window, cx| {
                                    crate::modals::create_instance::open_create_instance(
                                        this.data.metadata.clone(),
                                        this.data.instances.clone(),
                                        this.backend_handle.clone(),
                                        window,
                                        cx,
                                    );
                                })),
                        ),
                )
                .into_any_element();
        }

        self.ensure_recommendations_loaded(&instances, cx);

        let stats = self.aggregate_stats(&instances, cx);
        let last_played = instances.first().cloned();
        let showcase = instances.iter().take(4).cloned().collect::<Vec<_>>();
        let hide_usernames = InterfaceConfig::get(cx).hide_usernames;
        let modrinth_favorites = InterfaceConfig::get(cx).modrinth_favorites.clone();
        let curseforge_favorites = InterfaceConfig::get(cx).curseforge_favorites.clone();
        let account_name = self
            .data
            .accounts
            .read(cx)
            .selected_account
            .as_ref()
            .map(|account| account.username(hide_usernames))
            .unwrap_or_else(|| t::home::guest().into());

        let refresh_generation = self.refresh_generation;

        let hero = self.hero_section(last_played.clone(), &account_name, refresh_generation, cx);
        let stats_panel = self.stats_section(&stats, cx);
        let modpacks = self.modpack_section(&showcase, cx);
        let favorites = self.favorites_section(modrinth_favorites, curseforge_favorites, cx);
        let recommendations = self.recommended_mods_section(cx);

        v_flex()
            .size_full()
            .p_6()
            .gap_6()
            .child(hero)
            .child(stats_panel)
            .child(modpacks)
            .child(favorites)
            .child(recommendations)
            .into_any_element()
    }
}

impl HomePage {
    fn hero_section(
        &self,
        last_played: Option<InstanceEntry>,
        account_name: &SharedString,
        refresh_generation: u64,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let muted = theme.muted;
        let muted_foreground = theme.muted_foreground;
        let border = theme.border;
        let sidebar_primary = theme.sidebar_primary;
        let sidebar = theme.sidebar;
        let secondary = theme.secondary;
        let radius = theme.radius;
        let radius_lg = theme.radius_lg;
        let _ = theme;
        let recent_skins = self.resolve_recent_skins(cx);
        let account_fingerprint = skin_fingerprint(&self.account_skin);
        let show_skin_empty_hint = recent_skins.len() <= 1 && InterfaceConfig::get(cx).recent_skins.is_empty();

        let mut skin_grid_items = Vec::new();
        for (index, (skin, variant)) in recent_skins.into_iter().enumerate() {
            let fingerprint = skin_fingerprint(&skin);
            let selected = fingerprint == account_fingerprint;
            let thumb_w = px(56.0);
            let thumb_h = px(56.0);
            let skin_for_click = skin.clone();

            let thumb = self.skin_thumbnail_cache.update(cx, |cache, cx| {
                cache.get_or_queue(&skin, variant, cx)
            });

            let thumb_element = if let Some(img) = thumb {
                gpui::img(img).w(thumb_w).h(thumb_h).into_any_element()
            } else {
                Skeleton::new()
                    .w(thumb_w)
                    .h(thumb_h)
                    .bg(secondary)
                    .into_any_element()
            };

            skin_grid_items.push(
                div()
                    .id(("home-skin", index))
                    .w(thumb_w)
                    .h(thumb_h)
                    .rounded(radius)
                    .border_1()
                    .border_color(if selected { sidebar_primary } else { border })
                    .bg(secondary)
                    .overflow_hidden()
                    .cursor_pointer()
                    .hover(|this| this.border_color(sidebar_primary.opacity(0.8)))
                    .child(thumb_element)
                    .on_click(cx.listener(move |page, _, _, cx| {
                        page.account_skin = skin_for_click.clone();
                        page.account_skin_variant = variant;
                        page.player_model.update(cx, |widget, cx| {
                            widget.set_skin(cx, skin_for_click.clone(), variant);
                        });
                        InterfaceConfig::record_recent_skin(&skin_for_click, cx);
                        cx.notify();
                    }))
                    .into_any_element(),
            );
        }

        h_flex()
            .w_full()
            .gap_6()
            .flex_wrap()
            .child(
                v_flex()
                    .gap_3()
                    .min_w_48()
                    .child(
                        div()
                            .text_2xl()
                            .font_family(SharedString::new_static(MINECRAFT_FONT))
                            .child(t::home::greeting(account_name)),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted_foreground)
                            .child(t::home::subtitle()),
                    )
                    .child(
                        h_flex()
                            .gap_3()
                            .items_start()
                            .child(
                                div()
                                    .w_48()
                                    .h_56()
                                    .rounded(radius_lg)
                                    .border_1()
                                    .border_color(border)
                                    .bg(muted)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(self.player_model.clone()),
                            )
                            .child(
                                v_flex()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(muted_foreground)
                                            .child(t::home::recent_skins()),
                                    )
                                    .child(
                                        div()
                                            .grid()
                                            .grid_cols(2)
                                            .gap_2()
                                            .children(skin_grid_items),
                                    )
                                    .when(show_skin_empty_hint, |this| {
                                        this.child(
                                            div()
                                                .text_xs()
                                                .text_color(muted_foreground)
                                                .max_w_32()
                                                .child(t::home::no_recent_skins()),
                                        )
                                    }),
                            ),
                    ),
            )
            .child({
                let mut panel = v_flex()
                    .flex_1()
                    .min_w_72()
                    .p_5()
                    .gap_4()
                    .rounded(radius_lg)
                    .border_1()
                    .border_color(sidebar_primary.opacity(0.45))
                    .bg(sidebar);

                if let Some(instance) = last_played {
                    let loader_and_version = format!(
                        "{} {}",
                        instance.configuration.loader.pretty_name(),
                        instance.configuration.minecraft_version.as_str(),
                    );
                    let last_played_label = format_last_played(instance.playtime.last_played_unix_ms);
                    let is_active = instance_is_active(instance.status);
                    let id = instance.id;
                    let name = instance.name.clone();
                    let backend_handle = self.backend_handle.clone();

                    panel = panel
                        .child(div().text_sm().text_color(muted_foreground).child(t::home::quick_play()))
                        .child(div().text_xl().font_semibold().child(instance.name.clone()))
                        .child(div().text_sm().text_color(muted_foreground).child(loader_and_version))
                        .child(div().text_xs().text_color(muted_foreground.opacity(0.85)).child(last_played_label))
                        .child({
                            let play = if is_active {
                                Button::new("home_playing")
                                    .disabled(true)
                                    .label(t::instance::running())
                            } else {
                                Button::new("home_play")
                                    .success()
                                    .icon(QuartzIcon::Play)
                                    .label(t::home::play_now())
                                    .on_click(move |_, window, cx| {
                                        root::start_instance(id, name.clone(), None, &backend_handle, window, cx);
                                    })
                            };
                            h_flex()
                                .gap_2()
                                .child(play)
                                .child(
                                    Button::new("home_open_instance")
                                        .info()
                                        .label(t::instance::view())
                                        .on_click({
                                            let name = instance.name.clone();
                                            move |_, window, cx| {
                                                root::switch_page(
                                                    PageType::InstancePage { name: name.clone() },
                                                    &[PageType::Home, PageType::Instances],
                                                    window,
                                                    cx,
                                                );
                                            }
                                        }),
                                )
                        });
                } else {
                    panel = panel
                        .child(div().text_sm().text_color(muted_foreground).child(t::home::quick_play()))
                        .child(div().text_base().child(t::home::no_recent()));
                }

                panel.child(
                    h_flex()
                        .gap_1p5()
                        .items_center()
                        .text_xs()
                        .text_color(muted_foreground)
                        .child(animation::refresh_icon(refresh_generation))
                        .child(t::home::refresh_hint()),
                )
            })
            .into_any_element()
    }

    fn stats_section(&self, stats: &HomeStats, cx: &App) -> AnyElement {
        let theme = cx.theme();
        h_flex()
            .w_full()
            .gap_4()
            .flex_wrap()
            .child(stat_card(
                t::home::stat_instances(),
                stats.instance_count.to_string(),
                theme,
            ))
            .child(stat_card(
                t::instance::total_playtime(),
                format_playtime(stats.total_playtime_secs).to_string(),
                theme,
            ))
            .child(stat_card(
                t::home::stat_mods(),
                stats.total_mods.to_string(),
                theme,
            ))
            .into_any_element()
    }

    fn modpack_section(
        &self,
        instances: &[InstanceEntry],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut cards = Vec::new();
        for (index, instance) in instances.iter().enumerate() {
            cards.push(self.render_modpack_card(instance, index, cx));
        }

        let theme = cx.theme();
        let empty_radius = theme.radius;
        let empty_border = theme.border;
        let empty_muted = theme.muted_foreground;
        let is_empty = instances.is_empty();

        v_flex()
            .w_full()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(div().text_lg().font_semibold().child(t::home::your_modpacks()))
                    .child(
                        Button::new("home_all_instances")
                            .compact()
                            .small()
                            .info()
                            .label(t::home::view_all())
                            .on_click(|_, window, cx| {
                                root::switch_page(PageType::Instances, &[PageType::Home], window, cx);
                            }),
                    ),
            )
            .when(!is_empty, |this| {
                this.child(
                    div().child(
                        ResponsiveGrid::new(Size::new(AvailableSpace::Definite(px(280.0)), AvailableSpace::MinContent))
                            .w_full()
                            .gap_4()
                            .children(cards),
                    ),
                )
            })
            .when(is_empty, |this| {
                this.child(
                    div()
                        .p_4()
                        .rounded(empty_radius)
                        .border_1()
                        .border_color(empty_border)
                        .text_color(empty_muted)
                        .child(t::home::no_modpacks()),
                )
            })
            .into_any_element()
    }

    fn render_modpack_card(&self, instance: &InstanceEntry, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let card_radius = theme.radius_lg;
        let card_muted = theme.muted;
        let card_border = theme.border;
        let card_success = theme.success;
        let card_sidebar_primary = theme.sidebar_primary;
        let card_list_hover = theme.list_hover;
        let card_radius_sm = theme.radius;
        let card_muted_foreground = theme.muted_foreground;
        let loader_and_version = format!(
            "{} {}",
            instance.configuration.loader.pretty_name(),
            instance.configuration.minecraft_version.as_str(),
        );
        let last_played = format_last_played(instance.playtime.last_played_unix_ms);
        let is_active = instance_is_active(instance.status);

        let icon = if let Some(icon) = instance.icon.clone() {
            let transform = png_render_cache::ImageTransformation::Resize { width: 48, height: 48 };
            png_render_cache::render_with_transform(icon, transform, cx)
                .rounded(card_radius_sm)
                .size_12()
                .into_any_element()
        } else {
            let icon_path = instance
                .configuration
                .instance_fallback_icon
                .map(|s| s.as_str())
                .unwrap_or("icons/box.svg");
            Icon::default().path(icon_path).size_12().into_any_element()
        };

        let id = instance.id;
        let name = instance.name.clone();
        let backend_handle = self.backend_handle.clone();

        v_flex()
            .id(("home-modpack", index))
            .flex_1()
            .p_3()
            .gap_2()
            .min_w_64()
            .bg(card_muted)
            .border_1()
            .border_color(if is_active {
                card_success.opacity(0.55)
            } else {
                card_border
            })
            .rounded(card_radius)
            .hover(|this| this.border_color(card_sidebar_primary.opacity(0.85)).bg(card_list_hover))
            .on_click({
                let name = name.clone();
                move |_, window, cx| {
                    root::switch_page(
                        PageType::InstancePage { name: name.clone() },
                        &[PageType::Home],
                        window,
                        cx,
                    );
                }
            })
            .child(
                h_flex()
                    .gap_3()
                    .child(icon)
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_0p5()
                            .child(instance.name.clone())
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(card_muted_foreground)
                                    .child(loader_and_version),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(card_muted_foreground.opacity(0.85))
                                    .child(last_played),
                            ),
                    ),
            )
            .child(
                Button::new(("home-modpack-play", index))
                    .success()
                    .small()
                    .flex_1()
                    .label(if is_active {
                        t::instance::running()
                    } else {
                        t::instance::start::label()
                    })
                    .disabled(is_active)
                    .on_click(move |_, window, cx| {
                        if !is_active {
                            root::start_instance(id, name.clone(), None, &backend_handle, window, cx);
                        }
                    }),
            )
            .into_any_element()
    }

    fn favorites_section(
        &self,
        modrinth_favorites: Vec<ModrinthFavorite>,
        curseforge_favorites: Vec<CurseforgeFavorite>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut cards = Vec::new();

        for favorite in modrinth_favorites.into_iter().take(6) {
            cards.push(ModCard {
                title: favorite.title.clone().into(),
                subtitle: favorite.author.clone().into(),
                thumbnail: favorite.icon_url.map(SharedString::from),
                page: PageType::ModrinthProject {
                    project_id: favorite.project_id.clone().into(),
                    project_title: favorite.title.clone().into(),
                    install_for: None,
                },
            });
        }

        for favorite in curseforge_favorites.into_iter().take(6) {
            if cards.len() >= 6 {
                break;
            }
            cards.push(ModCard {
                title: favorite.name.clone().into(),
                subtitle: favorite.summary.clone().into(),
                thumbnail: favorite.thumbnail_url.map(SharedString::from),
                page: PageType::Curseforge { installing_for: None },
            });
        }

        self.render_mod_cards_section(
            t::home::favorite_mods(),
            t::home::no_favorites(),
            SharedString::from("home_favorites"),
            SharedString::from("home_view_favorites"),
            |_, window, cx| {
                InterfaceConfig::get_mut(cx).modrinth_favorites_only = true;
                root::switch_page(PageType::Modrinth { installing_for: None }, &[PageType::Home], window, cx);
            },
            cards,
            cx,
        )
    }

    fn recommended_mods_section(&self, cx: &mut Context<Self>) -> AnyElement {
        let cards: Vec<ModCard> = self
            .recommended_hits
            .iter()
            .map(|hit| {
                let project_id = hit.project_id.to_string();
                let project_title = hit.title.as_deref().unwrap_or("").to_string();
                ModCard {
                    title: project_title.clone().into(),
                    subtitle: hit.author.to_string().into(),
                    thumbnail: hit.icon_url.clone().map(|url| SharedString::from(url.to_string())),
                    page: PageType::ModrinthProject {
                        project_id: project_id.into(),
                        project_title: project_title.into(),
                        install_for: None,
                    },
                }
            })
            .collect();

        let mut section = self.render_mod_cards_section(
            t::home::recommended_mods(),
            t::home::no_recommendations(),
            SharedString::from("home_recommended"),
            SharedString::from("home_browse_mods"),
            |_, window, cx| {
                root::switch_page(PageType::Modrinth { installing_for: None }, &[PageType::Home], window, cx);
            },
            cards,
            cx,
        );

        if self.recommendations_loading {
            let theme = cx.theme();
            section = v_flex()
                .w_full()
                .gap_3()
                .child(section)
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(t::home::loading_recommendations()),
                )
                .into_any_element();
        } else if let Some(error) = &self.recommendations_error {
            let theme = cx.theme();
            section = v_flex()
                .w_full()
                .gap_3()
                .child(section)
                .child(div().text_sm().text_color(theme.muted_foreground).child(error.clone()))
                .into_any_element();
        }

        section
    }

    fn render_mod_cards_section(
        &self,
        title: impl Into<SharedString>,
        empty_message: impl Into<SharedString>,
        section_id: SharedString,
        action_id: SharedString,
        on_action: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        cards: Vec<ModCard>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let card_elements = cards
            .into_iter()
            .enumerate()
            .map(|(index, card)| card.render(section_id.clone(), index, cx))
            .collect::<Vec<_>>();

        let theme = cx.theme();
        let empty_radius = theme.radius;
        let empty_border = theme.border;
        let empty_muted = theme.muted_foreground;
        let has_cards = !card_elements.is_empty();
        let title = title.into();
        let empty_message = empty_message.into();
        let action_button_label = if section_id.as_ref() == "home_favorites" {
            t::home::view_favorites()
        } else {
            t::home::browse_mods()
        };

        v_flex()
            .w_full()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(div().text_lg().font_semibold().child(title))
                    .child(
                        Button::new(action_id)
                            .compact()
                            .small()
                            .info()
                            .label(action_button_label)
                            .on_click(on_action),
                    ),
            )
            .when(!has_cards, |this| {
                this.child(
                    div()
                        .p_4()
                        .rounded(empty_radius)
                        .border_1()
                        .border_color(empty_border)
                        .text_color(empty_muted)
                        .child(empty_message),
                )
            })
            .when(has_cards, |this| {
                this.child(
                    ResponsiveGrid::new(Size::new(AvailableSpace::Definite(px(220.0)), AvailableSpace::MinContent))
                        .w_full()
                        .gap_3()
                        .children(card_elements),
                )
            })
            .into_any_element()
    }
}

struct ModCard {
    title: SharedString,
    subtitle: SharedString,
    thumbnail: Option<SharedString>,
    page: PageType,
}

impl ModCard {
    fn render(self, section_id: SharedString, index: usize, cx: &mut App) -> AnyElement {
        let theme = cx.theme();
        let card_muted = theme.muted;
        let card_border = theme.border;
        let card_radius = theme.radius;
        let card_secondary = theme.secondary;
        let card_list_hover = theme.list_hover;
        let card_sidebar_primary = theme.sidebar_primary;
        let card_muted_foreground = theme.muted_foreground;
        let page = self.page;
        h_flex()
            .id((section_id, index))
            .gap_3()
            .p_3()
            .min_w_48()
            .bg(card_muted)
            .border_1()
            .border_color(card_border)
            .rounded(card_radius)
            .hover(|this| this.bg(card_list_hover).border_color(card_sidebar_primary.opacity(0.6)))
            .cursor_pointer()
            .on_click(move |_, window, cx| {
                root::switch_page(page.clone(), &[PageType::Home], window, cx);
            })
            .child({
                if let Some(url) = self.thumbnail {
                    gpui::img(SharedUri::from(url))
                        .size_10()
                        .rounded(card_radius)
                        .into_any_element()
                } else {
                    div()
                        .size_10()
                        .rounded(card_radius)
                        .bg(card_secondary)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(Icon::new(QuartzIcon::Box).size_4())
                        .into_any_element()
                }
            })
            .child(
                v_flex()
                    .flex_1()
                    .gap_0p5()
                    .overflow_hidden()
                    .child(div().text_sm().font_medium().truncate().child(self.title))
                    .child(
                        div()
                            .text_xs()
                            .text_color(card_muted_foreground)
                            .line_clamp(2)
                            .child(self.subtitle),
                    ),
            )
            .into_any_element()
    }
}

fn stat_card(label: impl Into<SharedString>, value: impl Into<SharedString>, theme: &gpui_component::Theme) -> impl IntoElement {
    v_flex()
        .flex_1()
        .min_w_40()
        .p_4()
        .gap_1()
        .rounded(theme.radius_lg)
        .border_1()
        .border_color(theme.border)
        .bg(theme.muted)
        .child(div().text_sm().text_color(theme.muted_foreground).child(label.into()))
        .child(div().text_xl().font_semibold().child(value.into()))
}
