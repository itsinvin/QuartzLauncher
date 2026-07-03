use std::{path::{Path, PathBuf}, sync::Arc};

use bridge::{
    handle::BackendHandle, install::{ContentDownload, ContentInstall, ContentInstallFile, InstallTarget}, instance::{ContentFolder, InstanceContentSummary, InstanceID}, message::MessageToBackend, meta::MetadataRequest
};
use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme as _, IndexPath, Sizable, StyledExt, WindowExt, button::{Button, ButtonVariants}, h_flex, input::SelectAll, list::ListState, notification::{Notification, NotificationType}, select::{Select, SelectEvent, SelectState}, switch::Switch, v_flex
};
use schema::{content::{ContentInstallReason, ContentSource}, curseforge::CurseforgeClassId, loader::Loader, modrinth::ModrinthHit};
use ustr::Ustr;

use crate::{
    component::{
        content_list::ContentListDelegate,
        named_dropdown::{NamedDropdown, NamedDropdownItem},
        recommendation_cards::{RecommendationCard, recommendation_section},
    },
    content_conflicts::detect_conflicts,
    entity::{
        instance::{ContentStates, InstanceEntry},
        metadata::{AsMetadataResult, FrontendMetadata, FrontendMetadataResult},
    },
    home_recommendations::{
        RecommendationContext, RecommendedContentKind, build_search_request, rank_recommendations,
    },
    icon::QuartzIcon,
    interface_config::{InstanceContentSortKey, InterfaceConfig},
    root,
    ui::PageType,
};

pub struct InstanceContentSubpage {
    content_type: ContentType,
    instance: InstanceID,
    instance_loader: Loader,
    instance_version: Ustr,
    instance_name: SharedString,
    backend_handle: BackendHandle,
    metadata: Entity<FrontendMetadata>,
    content_states: ContentStates,
    content_list: Entity<ListState<ContentListDelegate>>,
    content: Entity<Arc<[InstanceContentSummary]>>,
    sort_dropdown: Entity<SelectState<NamedDropdown<InstanceContentSortKey>>>,
    refresh_generation: u64,
    recommended_hits: Vec<ModrinthHit>,
    recommendations_loading: bool,
    recommendations_error: Option<SharedString>,
    recommendations_generation: u64,
    _recommendations_subscription: Option<Subscription>,
    _add_from_file_task: Option<Task<()>>,
}

#[derive(Clone, Copy)]
pub enum ContentType {
    Mods,
    ResourcePacks,
    Shaders,
}

impl From<ContentType> for u8 {
    fn from(value: ContentType) -> Self {
        match value {
            ContentType::Mods => 0,
            ContentType::ResourcePacks => 1,
            ContentType::Shaders => 2,
        }
    }
}

impl ContentType {
    fn content_folder(self) -> ContentFolder {
        match self {
            ContentType::Mods => ContentFolder::Mods,
            ContentType::ResourcePacks => ContentFolder::ResourcePacks,
            ContentType::Shaders => ContentFolder::Shaders,
        }
    }

    fn title(self) -> &'static str {
        match self {
            ContentType::Mods => t::instance::content::mods(),
            ContentType::ResourcePacks => t::instance::content::resourcepacks(),
            ContentType::Shaders => t::instance::content::shaders(),
        }
    }

    fn install_select(self) -> &'static str {
        match self {
            ContentType::Mods => t::instance::content::install::select_mods(),
            ContentType::ResourcePacks => t::instance::content::install::select_resourcepacks(),
            ContentType::Shaders => t::instance::content::install::select_shaders(),
        }
    }

    fn recommended_kind(self) -> RecommendedContentKind {
        match self {
            ContentType::Mods => RecommendedContentKind::Mod,
            ContentType::ResourcePacks => RecommendedContentKind::ResourcePack,
            ContentType::Shaders => RecommendedContentKind::Shader,
        }
    }

    fn recommended_title(self) -> &'static str {
        match self {
            ContentType::Mods => t::instance::content::recommended::mods(),
            ContentType::ResourcePacks => t::instance::content::recommended::resourcepacks(),
            ContentType::Shaders => t::instance::content::recommended::shaders(),
        }
    }

    fn modrinth_project_type(self) -> schema::modrinth::ModrinthProjectType {
        match self {
            ContentType::Mods => schema::modrinth::ModrinthProjectType::Mod,
            ContentType::ResourcePacks => schema::modrinth::ModrinthProjectType::Resourcepack,
            ContentType::Shaders => schema::modrinth::ModrinthProjectType::Shader,
        }
    }

    fn curseforge_class_id(self) -> CurseforgeClassId {
        match self {
            ContentType::Mods => CurseforgeClassId::Mod,
            ContentType::ResourcePacks => CurseforgeClassId::Resourcepack,
            ContentType::Shaders => CurseforgeClassId::Shader,
        }
    }

    fn valid_sort_modes(self) -> &'static [InstanceContentSortKey] {
        match self {
            ContentType::Mods => &[
                InstanceContentSortKey::Name,
                InstanceContentSortKey::ModId,
                InstanceContentSortKey::Filename,
                InstanceContentSortKey::ModifiedTime,
                InstanceContentSortKey::FileSize,
            ],
            ContentType::ResourcePacks | ContentType::Shaders => &[
                InstanceContentSortKey::Filename,
                InstanceContentSortKey::ModifiedTime,
                InstanceContentSortKey::FileSize,
            ],
        }
    }

    fn sort_key(self, config: &InterfaceConfig) -> InstanceContentSortKey {
        match self {
            ContentType::Mods => config.instance_mods_sort_key,
            ContentType::ResourcePacks => config.instance_resourcepacks_sort_key,
            ContentType::Shaders => config.instance_shaders_sort_key,
        }
    }

    fn sort_enabled_first(self, config: &InterfaceConfig) -> bool {
        match self {
            ContentType::Mods => config.instance_mods_sort_enabled_first,
            ContentType::ResourcePacks => config.instance_resourcepacks_sort_enabled_first,
            ContentType::Shaders => config.instance_shaders_sort_enabled_first,
        }
    }

    fn set_sort_key(self, config: &mut InterfaceConfig, value: InstanceContentSortKey) {
        match self {
            ContentType::Mods => config.instance_mods_sort_key = value,
            ContentType::ResourcePacks => config.instance_resourcepacks_sort_key = value,
            ContentType::Shaders => config.instance_shaders_sort_key = value,
        }
    }

    fn set_sort_enabled_first(self, config: &mut InterfaceConfig, value: bool) {
        match self {
            ContentType::Mods => config.instance_mods_sort_enabled_first = value,
            ContentType::ResourcePacks => config.instance_resourcepacks_sort_enabled_first = value,
            ContentType::Shaders => config.instance_shaders_sort_enabled_first = value,
        }
    }
}

impl InstanceContentSubpage {
    pub fn new(
        instance: &Entity<InstanceEntry>,
        content_type: ContentType,
        backend_handle: BackendHandle,
        metadata: Entity<FrontendMetadata>,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> Self {
        let instance = instance.read(cx);
        let instance_loader = instance.configuration.loader;
        let instance_version = instance.configuration.minecraft_version;
        let instance_id = instance.id;
        let instance_name = instance.name.clone();
        let content_states = instance.content_states.clone();

        let content_folder = content_type.content_folder();
        let content = instance.content[content_folder].clone();

        let config = InterfaceConfig::get(cx);
        let mut sort_key = content_type.sort_key(config);
        let enabled_first = content_type.sort_enabled_first(config);

        let valid_sort_modes = content_type.valid_sort_modes();
        if !valid_sort_modes.contains(&sort_key) {
            sort_key = valid_sort_modes[0];
        }

        let mut content_list_delegate = ContentListDelegate::new(instance_id, backend_handle.clone(), instance_loader, instance_version, sort_key, enabled_first);
        content_list_delegate.set_content(content.read(cx));
        content_list_delegate.set_conflicts(
            detect_conflicts(content.read(cx), content_folder, instance_loader).per_item,
        );

        let sort_dropdown = cx.new(|cx| {
            let items = valid_sort_modes.iter().map(|key| {
                NamedDropdownItem { name: key.name(), item: *key }
            }).collect::<Vec<_>>();

            let row = items.iter().position(|v| v.item == sort_key).unwrap_or(0);
            SelectState::new(NamedDropdown::new(items), Some(IndexPath::new(row)), window, cx)
        });

        let content_for_observe = content.clone();
        let content_list = cx.new(move |cx| {
            cx.observe(&content_for_observe, move |list: &mut ListState<ContentListDelegate>, content, cx| {
                let content = content.read(cx);
                list.delegate_mut().set_content(content);
                list.delegate_mut().set_conflicts(
                    detect_conflicts(content, content_folder, instance_loader).per_item,
                );
                cx.notify();
            }).detach();

            ListState::new(content_list_delegate, window, cx).selectable(false).searchable(true)
        });

        let content_for_recommendations = content.clone();
        cx.observe(&content_for_recommendations, |this, _, cx| {
            this.recommendations_generation = this.recommendations_generation.wrapping_add(1);
            this.recommended_hits.clear();
            this._recommendations_subscription = None;
            this.recommendations_loading = false;
            this.recommendations_error = None;
            cx.notify();
        }).detach();

        cx.subscribe(&sort_dropdown, |this, _, event: &SelectEvent<NamedDropdown<InstanceContentSortKey>>, cx| {
            let SelectEvent::Confirm(Some(value)) = event else {
                return;
            };

            let sort_key = value.item;
            let config = InterfaceConfig::get_mut(cx);

            if this.content_type.sort_key(config) == sort_key {
                return;
            }

            let enabled_first = this.content_type.sort_enabled_first(config);
            this.content_type.set_sort_key(config, sort_key);

            let content = this.content.read(cx).clone();
            let content_list = this.content_list.clone();
            cx.update_entity(&content_list, |list, cx| {
                list.delegate_mut().set_sort_options(sort_key, enabled_first);
                list.delegate_mut().set_content(&content);
                cx.notify();
            });
            cx.notify();
        }).detach();

        Self {
            content_type,
            instance: instance_id,
            instance_loader,
            instance_version,
            instance_name,
            backend_handle,
            metadata,
            content_states,
            content_list,
            content,
            sort_dropdown,
            refresh_generation: 0,
            recommended_hits: Vec::new(),
            recommendations_loading: false,
            recommendations_error: None,
            recommendations_generation: 0,
            _recommendations_subscription: None,
            _add_from_file_task: None,
        }
    }

    fn page_path(&self) -> Vec<PageType> {
        vec![
            PageType::Instances,
            PageType::InstancePage {
                name: self.instance_name.clone(),
            },
        ]
    }

    fn ensure_recommendations_loaded(&mut self, cx: &mut Context<Self>) {
        if self.recommendations_loading || self._recommendations_subscription.is_some() {
            return;
        }

        let kind = self.content_type.recommended_kind();
        let exclude_ids = InterfaceConfig::get(cx)
            .modrinth_favorites
            .iter()
            .map(|f| f.project_id.clone())
            .collect::<Vec<_>>();
        let ctx = RecommendationContext::from_installed_content(self.content.read(cx), &exclude_ids);
        let request = build_search_request(
            self.instance_loader,
            self.instance_version.as_str(),
            kind,
            &ctx,
        );
        let generation = self.recommendations_generation;

        self.recommendations_loading = true;
        self.recommendations_error = None;

        let data = FrontendMetadata::request(&self.metadata, MetadataRequest::ModrinthSearch(request), cx);
        let subscription = cx.observe(&data, move |page, data, cx| {
            let result: FrontendMetadataResult<schema::modrinth::ModrinthSearchResult> = data.read(cx).result();
            match result {
                FrontendMetadataResult::Loading => {}
                FrontendMetadataResult::Loaded(search_result) => {
                    if page.recommendations_generation != generation {
                        return;
                    }
                    let kind = page.content_type.recommended_kind();
                    let exclude_ids = InterfaceConfig::get(cx)
                        .modrinth_favorites
                        .iter()
                        .map(|f| f.project_id.clone())
                        .collect::<Vec<_>>();
                    let content = page.content.read(cx);
                    let ctx = RecommendationContext::from_installed_content(content, &exclude_ids);
                    page.recommended_hits =
                        rank_recommendations(&search_result.hits, kind, &ctx, 6);
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
            self.recommended_hits = rank_recommendations(&search_result.hits, kind, &ctx, 6);
            self.recommendations_loading = false;
            self._recommendations_subscription = None;
        } else if let FrontendMetadataResult::Error(error) = result {
            self.recommendations_error = Some(error);
            self.recommendations_loading = false;
            self._recommendations_subscription = None;
        }
    }

    fn install_paths(&self, paths: &[PathBuf], window: &mut Window, cx: &mut App) {
        let content_folder = self.content_type.content_folder().folder_name();

        let content_install = ContentInstall {
            target: InstallTarget::Instance(self.instance),
            loader: self.instance_loader,
            minecraft_version: self.instance_version,
            files: paths.into_iter().filter_map(|path| {
                Some(ContentInstallFile {
                    replace_old: None,
                    path: bridge::install::ContentInstallPath::Raw(Path::new(content_folder).join(path.file_name()?).into()),
                    download: ContentDownload::File { path: path.clone() },
                    content_source: ContentSource::Manual,
                    reason: ContentInstallReason::Standalone,
                })
            }).collect(),
        };
        crate::root::start_install(content_install, &self.backend_handle, window, cx);
    }
}

impl Render for InstanceContentSubpage {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        self.content_states.observe(self.content_type.content_folder());
        self.ensure_recommendations_loaded(cx);

        let (theme_border, theme_sidebar, theme_warning, theme_muted_foreground, theme_radius, theme_list_hover) = {
            let theme = cx.theme();
            (
                theme.border,
                theme.sidebar,
                theme.warning,
                theme.muted_foreground,
                theme.radius,
                theme.list_hover,
            )
        };

        let page_path = self.page_path();

        let header = h_flex()
            .gap_3()
            .p_3()
            .rounded_lg()
            .border_1()
            .border_color(theme_border)
            .bg(theme_sidebar)
            .child(div().text_xl().line_height(relative(1.35)).child(self.content_type.title()))
            .child({
                let refresh_generation = self.refresh_generation;
                div()
                    .id(format!("refresh-{}", u8::from(self.content_type)))
                    .cursor_pointer()
                    .flex()
                    .flex_row()
                    .items_center()
                    .px_2()
                    .py_1()
                    .rounded(theme_radius)
                    .border_1()
                    .border_color(theme_border)
                    .hover(|this| this.bg(theme_list_hover))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.refresh_generation = this.refresh_generation.wrapping_add(1);
                        this.recommendations_generation = this.recommendations_generation.wrapping_add(1);
                        this.recommended_hits.clear();
                        this._recommendations_subscription = None;
                        this.recommendations_loading = false;
                        this.backend_handle.send(MessageToBackend::ReloadContentFolder {
                            id: this.instance,
                            content_folder: this.content_type.content_folder(),
                        });
                        cx.notify();
                    }))
                    .child(h_flex().gap_1p5().items_center()
                        .child(crate::component::animation::refresh_icon(refresh_generation))
                        .child(div().text_sm().child(t::instance::content::refresh())))
            })
            .child(Button::new("update").label(t::instance::content::update::check::label(false)).success().compact().small().on_click({
                let backend_handle = self.backend_handle.clone();
                let instance_id = self.instance;
                move |_, window, cx| {
                    crate::root::start_update_check(instance_id, &backend_handle, window, cx);
                }
            }))
            .child(Button::new("addmr").label(t::instance::content::install::from_modrinth()).success().compact().small().on_click({
                let instance_name = self.instance_name.clone();
                let project_type = self.content_type.modrinth_project_type();
                move |_, window, cx| {
                    let page = crate::ui::PageType::Modrinth { installing_for: Some(instance_name.clone()) };
                    InterfaceConfig::get_mut(cx).modrinth_page_project_type = project_type;
                    let path = &[PageType::Instances, PageType::InstancePage { name: instance_name.clone() }];
                    root::switch_page(page, path, window, cx);
                }
            }))
            .child(Button::new("addcf").label(t::instance::content::install::from_curseforge()).success().compact().small().on_click({
                let instance_name = self.instance_name.clone();
                let class_id = self.content_type.curseforge_class_id();
                move |_, window, cx| {
                    let page = crate::ui::PageType::Curseforge { installing_for: Some(instance_name.clone()) };
                    InterfaceConfig::get_mut(cx).curseforge_page_class_id = class_id;
                    let path = &[PageType::Instances, PageType::InstancePage { name: instance_name.clone() }];
                    root::switch_page(page, path, window, cx);
                }
            }))
            .child(Button::new("addfile").label(t::instance::content::install::from_file()).success().compact().small().on_click({
                cx.listener(move |this, _, window, cx| {
                    let receiver = cx.prompt_for_paths(PathPromptOptions {
                        files: true,
                        directories: false,
                        multiple: true,
                        prompt: Some(this.content_type.install_select().into())
                    });

                    let entity = cx.entity();
                    let add_from_file_task = window.spawn(cx, async move |cx| {
                        let Ok(result) = receiver.await else {
                            return;
                        };
                        _ = cx.update_window_entity(&entity, move |this, window, cx| {
                            match result {
                                Ok(Some(paths)) => {
                                    this.install_paths(&paths, window, cx);
                                },
                                Ok(None) => {},
                                Err(error) => {
                                    let error = format!("{}", error);
                                    let notification = Notification::new()
                                        .autohide(false)
                                        .with_type(NotificationType::Error)
                                        .title(error);
                                    window.push_notification(notification, cx);
                                },
                            }
                        });
                    });
                    this._add_from_file_task = Some(add_from_file_task);
                })
            }));

        let content_items = self.content.read(cx);
        let conflicts = detect_conflicts(
            content_items,
            self.content_type.content_folder(),
            self.instance_loader,
        );

        let conflict_panel = if conflicts.messages.is_empty() {
            None
        } else {
            Some(
                v_flex()
                    .gap_1()
                    .p_3()
                    .rounded_lg()
                    .border_1()
                    .border_color(theme_warning)
                    .bg(theme_warning.opacity(0.08))
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(QuartzIcon::TriangleAlert)
                            .child(div().text_base().font_medium().child(t::instance::content::conflicts::title()))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme_muted_foreground)
                                    .child(t::instance::content::conflicts::summary(conflicts.messages.len())),
                            ),
                    )
                    .children(conflicts.messages.into_iter().map(|message| {
                        div().text_sm().pl_6().child(message)
                    })),
            )
        };

        let filter_bar_controls = h_flex()
            .cursor_default()
            .block_mouse_except_scroll()
            .gap_3()
            .items_center()
            .px_3()
            .py_1()
            .rounded_lg()
            .border_1()
            .border_color(theme_border)
            .bg(theme_sidebar)
            .child(div().child(Select::new(&self.sort_dropdown).small().title_prefix("Sort: ")))
            .child(h_flex().gap_1()
                .child(div().text_sm().child("Enabled first"))
                .child(Switch::new("enabled_first")
                    .checked(self.content_type.sort_enabled_first(InterfaceConfig::get(cx)))
                    .on_click(cx.listener(|this, checked, _, cx| {
                        let config = InterfaceConfig::get_mut(cx);
                        let enabled_first = *checked;

                        if this.content_type.sort_enabled_first(config) == enabled_first {
                            return;
                        }

                        let sort_key = this.content_type.sort_key(config);
                        this.content_type.set_sort_enabled_first(config, enabled_first);

                        let content = this.content.read(cx).clone();
                        let content_list = this.content_list.clone();
                        cx.update_entity(&content_list, |list, cx| {
                            list.delegate_mut().set_sort_options(sort_key, enabled_first);
                            list.delegate_mut().set_content(&content);
                            cx.notify();
                        });
                        cx.notify();
                    }))
                )
            )
            .absolute()
            .top(px(8.0))
            .right(px(12.0));

        let install_for = Some(self.instance_name.clone());
        let recommendation_cards: Vec<RecommendationCard> = self
            .recommended_hits
            .iter()
            .map(|hit| RecommendationCard::from_modrinth_hit(hit, install_for.clone(), &page_path))
            .collect();

        let recommendations = {
            let content_type = self.content_type;
            let instance_name = self.instance_name.clone();
            recommendation_section(
                self.content_type.recommended_title(),
                t::instance::content::recommended::empty(),
                t::instance::content::recommended::browse(),
                SharedString::from(format!("instance_recommended_{}", u8::from(self.content_type))),
                SharedString::from(format!("instance_browse_{}", u8::from(self.content_type))),
                recommendation_cards,
                &page_path,
                move |_, window, cx| {
                    InterfaceConfig::get_mut(cx).modrinth_page_project_type = content_type.modrinth_project_type();
                    let page = PageType::Modrinth {
                        installing_for: Some(instance_name.clone()),
                    };
                    let path = vec![
                        PageType::Instances,
                        PageType::InstancePage {
                            name: instance_name.clone(),
                        },
                    ];
                    root::switch_page(page, &path, window, cx);
                },
                self.recommendations_loading
                    .then(|| t::instance::content::recommended::loading().into()),
                self.recommendations_error.clone(),
                cx,
            )
        };

        v_flex().p_4().gap_3().size_full()
            .child(header)
            .when_some(conflict_panel, |this, panel| this.child(panel))
            .child(
                div()
                    .id("content-list-area")
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .drag_over(|style, _: &ExternalPaths, _, cx| {
                        style.bg(cx.theme().accent)
                    })
                    .on_drop(cx.listener(|this, paths: &ExternalPaths, window, cx| {
                        this.install_paths(paths.paths(), window, cx);
                    }))
                    .size_full()
                    .border_1()
                    .rounded_lg()
                    .border_color(theme_border)
                    .bg(theme_sidebar)
                    .child(self.content_list.clone())
                    .child(filter_bar_controls)
                    .on_click({
                        let content_list = self.content_list.clone();
                        move |_, _, cx| {
                            cx.update_entity(&content_list, |list, cx| {
                                list.delegate_mut().clear_selection();
                                cx.notify();
                            })
                        }
                    })
                    .key_context("Input")
                    .on_action({
                        let content_list = self.content_list.clone();
                        move |_: &SelectAll, _, cx| {
                            cx.update_entity(&content_list, |list, cx| {
                                list.delegate_mut().select_all();
                                cx.notify();
                            })
                        }
                    }),
            )
            .child(recommendations)
    }
}
