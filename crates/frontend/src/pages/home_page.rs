    handle::BackendHandle,
    instance::{ContentFolder, InstanceStatus},
    message::{AccountSkinResult, MessageToBackend},
};
use gpui::{prelude::FluentBuilder as _, *};
use gpui_component::{
    ActiveTheme as _, Icon, Sizable, WindowExt,
    button::{Button, ButtonVariants},
    h_flex, v_flex,
};

use once_cell::sync::Lazy;
use schema::unique_bytes::UniqueBytes;

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
    },
    icon::QuartzIcon,
    interface_config::InterfaceConfig,
    pages::page::Page,
    png_render_cache,
    root,
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
    account_skin: UniqueBytes,
    request_account_skin: Option<Task<()>>,
    refresh_generation: u64,
    _instance_added_subscription: Subscription,
    _instance_modified_subscription: Subscription,
    _instance_removed_subscription: Subscription,
}

impl HomePage {
    pub fn new(data: &DataEntities, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let player_model = cx.new(|cx| PlayerModelWidget::new(cx, DEFAULT_SKIN.clone()));
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
            account_skin: DEFAULT_SKIN.clone(),
            request_account_skin: None,
            refresh_generation: 0,
            _instance_added_subscription,
            _instance_modified_subscription,
            _instance_removed_subscription,
        };
        page.load_account_skin(window, cx);
        page
    }

    fn load_account_skin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

        self.request_account_skin = Some(window.spawn(cx, async move |page, cx| {
            let Ok(result) = recv.await else {
                return;
            };
            let _ = page.update(cx, |page, cx| {
                if let AccountSkinResult::Success { skin, variant } = result {
                    if let Some(skin) = skin {
                        page.account_skin = skin.clone();
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
            .on_click(cx.listener(|this, _, window, cx| {
                this.refresh_generation = this.refresh_generation.wrapping_add(1);
                this.load_account_skin(window, cx);
                cx.notify();
            }))
    }

    fn scrollable(&self, _cx: &App) -> bool {
        true
    }
}

impl Render for HomePage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let instances = self.sorted_instances(cx);

        if instances.is_empty() {
            let logo_scale = animation::animated_logo_scale(window, cx);
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

        let stats = self.aggregate_stats(&instances, cx);
        let last_played = instances.first().cloned();
        let showcase = instances.iter().take(4).cloned().collect::<Vec<_>>();
        let config = InterfaceConfig::get(cx);
        let account_name = self
            .data
            .accounts
            .read(cx)
            .selected_account
            .as_ref()
            .map(|account| account.username(config.hide_usernames))
            .unwrap_or_else(|| t::home::guest().into());

        let refresh_generation = self.refresh_generation;

        v_flex()
            .size_full()
            .p_6()
            .gap_6()
            .child(self.hero_section(last_played.as_ref(), &account_name, refresh_generation, window, cx))
            .child(self.stats_section(&stats, cx))
            .child(self.modpack_section(&showcase, window, cx))
            .child(self.recommendations_section(config, window, cx))
            .into_any_element()
    }
}

impl HomePage {
    fn hero_section(
        &self,
        last_played: Option<&InstanceEntry>,
        account_name: &SharedString,
        refresh_generation: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();

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
                            .text_color(theme.muted_foreground)
                            .child(t::home::subtitle()),
                    )
                    .child(
                        div()
                            .w_40()
                            .h_48()
                            .rounded(theme.radius_lg)
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.muted)
                            .overflow_hidden()
                            .child(self.player_model.clone()),
                    ),
            )
            .child({
                let mut panel = v_flex()
                    .flex_1()
                    .min_w_72()
                    .p_5()
                    .gap_4()
                    .rounded(theme.radius_lg)
                    .border_1()
                    .border_color(theme.sidebar_primary.opacity(0.45))
                    .bg(theme.sidebar);

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
                        .child(div().text_sm().text_color(theme.muted_foreground).child(t::home::quick_play()))
                        .child(div().text_xl().font_semibold().child(instance.name.clone()))
                        .child(div().text_sm().text_color(theme.muted_foreground).child(loader_and_version))
                        .child(div().text_xs().text_color(theme.muted_foreground.opacity(0.85)).child(last_played_label))
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
                        .child(div().text_sm().text_color(theme.muted_foreground).child(t::home::quick_play()))
                        .child(div().text_base().child(t::home::no_recent()));
                }

                panel.child(
                    h_flex()
                        .gap_1p5()
                        .items_center()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(animation::refresh_icon(refresh_generation))
                        .child(t::home::refresh_hint()),
                )
            })
    }

    fn stats_section(&self, stats: &HomeStats, cx: &App) -> impl IntoElement {
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
    }

    fn modpack_section(
        &self,
        instances: &[InstanceEntry],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let cards = instances
            .iter()
            .enumerate()
            .map(|(index, instance)| self.render_modpack_card(instance, index, cx))
            .collect::<Vec<_>>();

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
            .child(
                div().child(
                    ResponsiveGrid::new(Size::new(AvailableSpace::Definite(px(280.0)), AvailableSpace::MinContent))
                        .w_full()
                        .gap_4()
                        .children(cards),
                ),
            )
            .when(instances.is_empty(), |this| {
                this.child(
                    div()
                        .p_4()
                        .rounded(theme.radius)
                        .border_1()
                        .border_color(theme.border)
                        .text_color(theme.muted_foreground)
                        .child(t::home::no_modpacks()),
                )
            })
    }

    fn render_modpack_card(&self, instance: &InstanceEntry, index: usize, cx: &App) -> AnyElement {
        let theme = cx.theme();
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
                .rounded(theme.radius)
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
            .bg(theme.muted)
            .border_1()
            .border_color(if is_active {
                theme.success.opacity(0.55)
            } else {
                theme.border
            })
            .rounded(theme.radius_lg)
            .hover(|this| this.border_color(theme.sidebar_primary.opacity(0.85)).bg(theme.list_hover))
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
                                    .text_color(theme.muted_foreground)
                                    .child(loader_and_version),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground.opacity(0.85))
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

    fn recommendations_section(
        &self,
        config: &InterfaceConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let mut recommendations = Vec::new();

        for favorite in config.modrinth_favorites.iter().take(4) {
            recommendations.push(RecommendationCard {
                title: favorite.title.clone().into(),
                subtitle: favorite.author.clone().into(),
                thumbnail: favorite.icon_url.as_deref().map(SharedString::from),
                page: PageType::ModrinthProject {
                    project_id: favorite.project_id.clone().into(),
                    project_title: favorite.title.clone().into(),
                    install_for: None,
                },
            });
        }

        for favorite in config.curseforge_favorites.iter().take(4) {
            if recommendations.len() >= 6 {
                break;
            }
            recommendations.push(RecommendationCard {
                title: favorite.name.clone().into(),
                subtitle: favorite.summary.clone().into(),
                thumbnail: favorite.thumbnail_url.as_deref().map(SharedString::from),
                page: PageType::Curseforge { installing_for: None },
            });
        }

        v_flex()
            .w_full()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(div().text_lg().font_semibold().child(t::home::recommended_mods()))
                    .child(
                        Button::new("home_browse_mods")
                            .compact()
                            .small()
                            .info()
                            .label(t::home::browse_mods())
                            .on_click(|_, window, cx| {
                                root::switch_page(PageType::Modrinth { installing_for: None }, &[PageType::Home], window, cx);
                            }),
                    ),
            )
            .when(recommendations.is_empty(), |this| {
                this.child(
                    div()
                        .p_4()
                        .rounded(theme.radius)
                        .border_1()
                        .border_color(theme.border)
                        .text_color(theme.muted_foreground)
                        .child(t::home::no_recommendations()),
                )
            })
            .when(!recommendations.is_empty(), |this| {
                this.child(
                    ResponsiveGrid::new(Size::new(AvailableSpace::Definite(px(220.0)), AvailableSpace::MinContent))
                        .w_full()
                        .gap_3()
                        .children(recommendations.into_iter().enumerate().map(|(index, card)| {
                            card.render(index, &theme, window, cx)
                        })),
                )
            })
    }
}

struct RecommendationCard {
    title: SharedString,
    subtitle: SharedString,
    thumbnail: Option<SharedString>,
    page: PageType,
}

impl RecommendationCard {
    fn render(self, index: usize, theme: &gpui_component::Theme, _window: &mut Window, _cx: &mut App) -> AnyElement {
        let page = self.page;
        h_flex()
            .id(("home-rec", index))
            .gap_3()
            .p_3()
            .min_w_52()
            .bg(theme.muted)
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius)
            .hover(|this| this.bg(theme.list_hover).border_color(theme.sidebar_primary.opacity(0.6)))
            .cursor_pointer()
            .on_click(move |_, window, cx| {
                root::switch_page(page.clone(), &[PageType::Home], window, cx);
            })
            .child({
                if let Some(url) = self.thumbnail {
                    gpui::img(ImageSource::Uri(SharedUri::from(url)))
                        .size_10()
                        .rounded(theme.radius)
                        .into_any_element()
                } else {
                    div()
                        .size_10()
                        .rounded(theme.radius)
                        .bg(theme.secondary)
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
                            .text_color(theme.muted_foreground)
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
