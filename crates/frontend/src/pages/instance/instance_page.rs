use bridge::{
    handle::BackendHandle,
    instance::{InstanceID, InstanceStatus},
    message::MessageToBackend,
};
use gpui::{prelude::*, *};
use gpui_component::{
    WindowExt, button::{Button, ButtonGroup, ButtonVariants}, h_flex, tab::{Tab, TabBar}, v_flex
};
use serde::{Deserialize, Serialize};

use crate::{
    entity::{DataEntities, instance::InstanceEntry}, icon::QuartzIcon, interface_config::InterfaceConfig, pages::{instance::{content_subpage::InstanceContentSubpage, logs_subpage::InstanceLogsSubpage, performance_subpage::InstancePerformanceSubpage, quickplay_subpage::InstanceQuickplaySubpage, settings_subpage::InstanceSettingsSubpage}, page::Page}, root,
};

use super::content_subpage::ContentType;

pub struct InstancePage {
    backend_handle: BackendHandle,
    data: DataEntities,
    pub instance: Entity<InstanceEntry>,
    subpages: InstanceSubpageCache,
}

impl InstancePage {
    pub fn new(instance_id: InstanceID, data: &DataEntities, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let instance = data.instances.read(cx).entries.get(&instance_id).unwrap().clone();

        Self {
            backend_handle: data.backend_handle.clone(),
            data: data.clone(),
            instance,
            subpages: InstanceSubpageCache::default(),
        }
    }
}

impl Page for InstancePage {
    fn controls(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let instance = self.instance.read(cx);
        let id = instance.id;
        let name = instance.name.clone();
        let backend_handle = self.backend_handle.clone();

        let button = match instance.status {
            InstanceStatus::NotRunning => {
                Button::new("start_instance").success().icon(QuartzIcon::Play).label(t::instance::start::label()).on_click(
                    move |_, window, cx| {
                        root::start_instance(id, name.clone(), None, &backend_handle, window, cx);
                    },
                ).into_any_element()
            },
            InstanceStatus::Launching => {
                Button::new("launching").warning().icon(QuartzIcon::Loader).label(t::instance::start::starting()).into_any_element()
            },
            InstanceStatus::Stopping => {
                Button::new("stopping")
                    .danger()
                    .icon(QuartzIcon::Loader)
                    .label(t::instance::start::stopping())
                    .on_click({
                        let backend_handle = backend_handle.clone();
                        move |_, _, _| {
                            backend_handle.send(MessageToBackend::KillInstance { id });
                        }
                    })
                    .into_any_element()
            },
            InstanceStatus::Running => {
                ButtonGroup::new("running")
                    .child(Button::new("kill_instance")
                        .danger()
                        .icon(QuartzIcon::Close)
                        .label(t::instance::kill_instance())
                        .on_click({
                            let backend_handle = backend_handle.clone();
                            move |_, _, _| {
                                backend_handle.send(MessageToBackend::KillInstance { id });
                            }
                        }))
                    .child(Button::new("start_again")
                        .success()
                        .icon(QuartzIcon::Play)
                        .on_click(move |_, window, cx| {
                            let name = name.clone();
                            let backend_handle = backend_handle.clone();
                            window.open_dialog(cx, move |dialog, _, _| {
                                dialog.title("Instance already running")
                                    .overlay_closable(false)
                                    .flex()
                                    .line_height(rems(1.2))
                                    .child("Starting it again may cause malfunction or corrupt your saved worlds.")
                                    .child(div().h_2())
                                    .child("We cannot take responsibility for any issues if you choose to start another game. Would you like to continue anyway?")
                                    .footer(h_flex()
                                        .gap_2()
                                        .w_full()
                                        .child(
                                            Button::new("cancel")
                                                .label("Cancel")
                                                .on_click(|_, window, cx| {
                                                    window.close_dialog(cx);
                                                }).flex_grow()
                                        )
                                        .child(
                                            Button::new("ok")
                                                .success()
                                                .label("Start anyway")
                                                .on_click({
                                                    let name = name.clone();
                                                    let backend_handle = backend_handle.clone();
                                                    move |_, window, cx| {
                                                        window.close_dialog(cx);
                                                        root::start_instance(id, name.clone(), None, &backend_handle, window, cx);
                                                    }
                                                })
                                        ))
                            })
                        })).into_any_element()
            },
        };

        let open_dot_minecraft_button = Button::new("open_dot_minecraft")
            .info()
            .icon(QuartzIcon::FolderOpen)
            .label(t::instance::open_folder())
            .on_click({
            let dot_minecraft = instance.dot_minecraft_folder.clone();
            move |_, window, cx| {
                crate::open_folder(&dot_minecraft, window, cx);
            }
        });

        h_flex().gap_3().child(button).child(open_dot_minecraft_button)
    }

    fn scrollable(&self, _cx: &App) -> bool {
        false
    }
}

impl Render for InstancePage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let instance_subpage = InterfaceConfig::get(cx).instance_subpage;
        let subpage = self.subpages.get_or_create(
            instance_subpage,
            &self.instance,
            &self.data,
            self.backend_handle.clone(),
            window,
            cx,
        );

        let show_shader_tab = self.instance.read(cx).configuration.show_shader_tab || matches!(subpage, InstanceSubpage::Shaders(_));

        let selected_index = match &subpage {
            InstanceSubpage::Quickplay(_) => 0,
            InstanceSubpage::Logs(_) => 1,
            InstanceSubpage::Mods(_) => 2,
            InstanceSubpage::ResourcePacks(_) => 3,
            InstanceSubpage::Shaders(_) => 4,
            InstanceSubpage::Performance(_) => if show_shader_tab { 5 } else { 4 },
            InstanceSubpage::Settings(_) => if show_shader_tab { 6 } else { 5 },
        };

        v_flex()
            .size_full()
            .child(
                TabBar::new("bar")
                    .prefix(div().w_4())
                    .selected_index(selected_index)
                    .underline()
                    .child(Tab::new().label(t::instance::quickplay()))
                    .child(Tab::new().label(t::instance::logs::title()))
                    .child(Tab::new().label(t::instance::content::mods()))
                    .child(Tab::new().label(t::instance::content::resourcepacks()))
                    .when(show_shader_tab, |this| {
                        this.child(Tab::new().label(t::instance::content::shaders()))
                    })
                    .child(Tab::new().label(t::instance::performance::title()))
                    .child(Tab::new().label(t::settings::title()))
                    .on_click(cx.listener(move |_, index, _, cx| {
                        let page_type = match *index {
                            0 => InstanceSubpageType::Quickplay,
                            1 => InstanceSubpageType::Logs,
                            2 => InstanceSubpageType::Mods,
                            3 => InstanceSubpageType::ResourcePacks,
                            4 => if show_shader_tab {
                                InstanceSubpageType::Shaders
                            } else {
                                InstanceSubpageType::Performance
                            },
                            5 => if show_shader_tab {
                                InstanceSubpageType::Performance
                            } else {
                                InstanceSubpageType::Settings
                            },
                            6 => {
                                if show_shader_tab {
                                    InstanceSubpageType::Settings
                                } else {
                                    return;
                                }
                            },
                            _ => {
                                return;
                            },
                        };
                        InterfaceConfig::get_mut(cx).instance_subpage = page_type;
                    })),
            )
            .child(subpage.into_any_element())
    }
}

#[derive(Default)]
struct InstanceSubpageCache {
    quickplay: Option<Entity<InstanceQuickplaySubpage>>,
    logs: Option<Entity<InstanceLogsSubpage>>,
    mods: Option<Entity<InstanceContentSubpage>>,
    resource_packs: Option<Entity<InstanceContentSubpage>>,
    shaders: Option<Entity<InstanceContentSubpage>>,
    performance: Option<Entity<InstancePerformanceSubpage>>,
    settings: Option<Entity<InstanceSettingsSubpage>>,
}

impl InstanceSubpageCache {
    fn get_or_create(
        &mut self,
        subpage_type: InstanceSubpageType,
        instance: &Entity<InstanceEntry>,
        data: &DataEntities,
        backend_handle: BackendHandle,
        window: &mut gpui::Window,
        cx: &mut App,
    ) -> InstanceSubpage {
        match subpage_type {
            InstanceSubpageType::Quickplay => {
                if self.quickplay.is_none() {
                    self.quickplay = Some(cx.new(|cx| {
                        InstanceQuickplaySubpage::new(instance, backend_handle.clone(), window, cx)
                    }));
                }
                InstanceSubpage::Quickplay(self.quickplay.clone().unwrap())
            },
            InstanceSubpageType::Logs => {
                if self.logs.is_none() {
                    self.logs = Some(cx.new(|cx| {
                        InstanceLogsSubpage::new(instance, backend_handle.clone(), window, cx)
                    }));
                }
                InstanceSubpage::Logs(self.logs.clone().unwrap())
            },
            InstanceSubpageType::Mods => {
                if self.mods.is_none() {
                    self.mods = Some(cx.new(|cx| {
                        InstanceContentSubpage::new(instance, ContentType::Mods, backend_handle.clone(), window, cx)
                    }));
                }
                InstanceSubpage::Mods(self.mods.clone().unwrap())
            },
            InstanceSubpageType::ResourcePacks => {
                if self.resource_packs.is_none() {
                    self.resource_packs = Some(cx.new(|cx| {
                        InstanceContentSubpage::new(instance, ContentType::ResourcePacks, backend_handle.clone(), window, cx)
                    }));
                }
                InstanceSubpage::ResourcePacks(self.resource_packs.clone().unwrap())
            },
            InstanceSubpageType::Shaders => {
                if self.shaders.is_none() {
                    self.shaders = Some(cx.new(|cx| {
                        InstanceContentSubpage::new(instance, ContentType::Shaders, backend_handle.clone(), window, cx)
                    }));
                }
                InstanceSubpage::Shaders(self.shaders.clone().unwrap())
            },
            InstanceSubpageType::Performance => {
                if self.performance.is_none() {
                    self.performance = Some(cx.new(|cx| {
                        InstancePerformanceSubpage::new(instance, window, cx)
                    }));
                }
                InstanceSubpage::Performance(self.performance.clone().unwrap())
            },
            InstanceSubpageType::Settings => {
                if self.settings.is_none() {
                    self.settings = Some(cx.new(|cx| {
                        InstanceSettingsSubpage::new(instance, data, backend_handle, window, cx)
                    }));
                }
                InstanceSubpage::Settings(self.settings.clone().unwrap())
            },
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceSubpageType {
    #[default]
    Quickplay,
    Logs,
    Mods,
    ResourcePacks,
    Shaders,
    Performance,
    Settings,
}

impl InstanceSubpageType {
    pub fn create(
        self,
        instance: &Entity<InstanceEntry>,
        data: &DataEntities,
        backend_handle: BackendHandle,
        window: &mut gpui::Window,
        cx: &mut App,
    ) -> InstanceSubpage {
        InstanceSubpageCache::default().get_or_create(self, instance, data, backend_handle, window, cx)
    }
}

#[derive(Clone)]
pub enum InstanceSubpage {
    Quickplay(Entity<InstanceQuickplaySubpage>),
    Logs(Entity<InstanceLogsSubpage>),
    Mods(Entity<InstanceContentSubpage>),
    ResourcePacks(Entity<InstanceContentSubpage>),
    Shaders(Entity<InstanceContentSubpage>),
    Performance(Entity<InstancePerformanceSubpage>),
    Settings(Entity<InstanceSettingsSubpage>),
}

impl InstanceSubpage {
    pub fn page_type(&self) -> InstanceSubpageType {
        match self {
            InstanceSubpage::Quickplay(_) => InstanceSubpageType::Quickplay,
            InstanceSubpage::Logs(_) => InstanceSubpageType::Logs,
            InstanceSubpage::Mods(_) => InstanceSubpageType::Mods,
            InstanceSubpage::ResourcePacks(_) => InstanceSubpageType::ResourcePacks,
            InstanceSubpage::Shaders(_) => InstanceSubpageType::Shaders,
            InstanceSubpage::Performance(_) => InstanceSubpageType::Performance,
            InstanceSubpage::Settings(_) => InstanceSubpageType::Settings,
        }
    }

    pub fn into_any_element(self) -> AnyElement {
        match self {
            Self::Quickplay(entity) => entity.into_any_element(),
            Self::Logs(entity) => entity.into_any_element(),
            Self::Mods(entity) => entity.into_any_element(),
            Self::ResourcePacks(entity) => entity.into_any_element(),
            Self::Shaders(entity) => entity.into_any_element(),
            Self::Performance(entity) => entity.into_any_element(),
            Self::Settings(entity) => entity.into_any_element(),
        }
    }
}
