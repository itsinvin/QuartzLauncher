use std::time::Duration;

use bridge::handle::BackendHandle;
use gpui::{prelude::*, *};
use gpui_component::{
    IndexPath, button::{Button, ButtonVariants}, h_flex, input::{Input, InputEvent, InputState}, select::{Select, SelectDelegate, SelectEvent, SelectItem, SelectState}, table::{DataTable, TableState}, v_flex, ActiveTheme, Sizable
};
use strum::IntoEnumIterator;

use crate::{
    component::{animation, instance_list::InstanceList, named_dropdown::{NamedDropdown, NamedDropdownItem}, quartz_logo::QuartzLogo, responsive_grid::ResponsiveGrid}, entity::{DataEntities, instance::InstanceEntries, metadata::FrontendMetadata}, icon::QuartzIcon, interface_config::{InstancesViewMode, InterfaceConfig}, pages::page::Page,
    modals, MINECRAFT_FONT,
};

pub struct InstancesPage {
    instance_table: Entity<TableState<InstanceList>>,
    view_dropdown: Entity<SelectState<NamedDropdown<InstancesViewMode>>>,
    search_state: Entity<InputState>,
    _search_subscription: Subscription,
    search_generation: usize,

    metadata: Entity<FrontendMetadata>,
    instances: Entity<InstanceEntries>,

    backend_handle: BackendHandle,
}

impl InstancesPage {
    pub fn new(data: &DataEntities, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let instance_table = InstanceList::create_table(data, window, cx);
        let view_dropdown = cx.new(|cx| {
            let items = InstancesViewMode::iter().map(|view| {
                NamedDropdownItem { name: view.name(), item: view }
            }).collect::<Vec<_>>();
            let current_view = InterfaceConfig::get(cx).instances_view_mode;
            let row = items.iter().position(|v| v.item == current_view).unwrap_or(0);
            let delegate = NamedDropdown::new(items);
            SelectState::new(delegate, Some(IndexPath::new(row)), window, cx)
        });
        cx.subscribe(&view_dropdown, |_, _, event: &SelectEvent<NamedDropdown<InstancesViewMode>>, cx| {
            let SelectEvent::Confirm(Some(value)) = event else {
                return;
            };
            let view = value.item;

            InterfaceConfig::get_mut(cx).instances_view_mode = view;
        }).detach();

        let search_state = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t::instance::search_placeholder())
        });
        let _search_subscription = cx.subscribe(&search_state, |this, state, event: &InputEvent, cx| {
            let InputEvent::Change = event else {
                return;
            };
            this.search_generation += 1;
            let generation = this.search_generation;
            let query = state.read(cx).text().to_string();
            let page_entity = cx.entity();
            cx.spawn(async move |_, cx| {
                cx.background_executor().timer(Duration::from_millis(250)).await;
                let _ = page_entity.update(cx, |page, cx| {
                    if page.search_generation != generation {
                        return;
                    }
                    page.instance_table.update(cx, |table, cx| {
                        table.delegate_mut().set_filter(query.into());
                        cx.notify();
                    });
                });
            }).detach();
        });

        Self {
            instance_table,
            view_dropdown,
            search_state,
            _search_subscription,
            search_generation: 0,
            metadata: data.metadata.clone(),
            instances: data.instances.clone(),
            backend_handle: data.backend_handle.clone(),
        }
    }
}

impl Page for InstancesPage {
    fn controls(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let create_instance = Button::new("create_instance")
            .success()
            .icon(QuartzIcon::Plus)
            .label(t::instance::create())
            .on_click(cx.listener(|this, _, window, cx| {
                modals::create_instance::open_create_instance(this.metadata.clone(), this.instances.clone(),
                    this.backend_handle.clone(), window, cx);
            }));
        let select_view = div()
            .child(Select::new(&self.view_dropdown).title_prefix(format!("{}: ", t::instance::view())));

        h_flex()
            .gap_3()
            .flex_1()
            .child(Input::new(&self.search_state).flex_1().min_w_48().small())
            .child(create_instance)
            .child(select_view)
    }

    fn scrollable(&self, cx: &App) -> bool {
        match InterfaceConfig::get(cx).instances_view_mode {
            InstancesViewMode::Cards => true,
            InstancesViewMode::List => false,
        }
    }
}

impl Render for InstancesPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let delegate = self.instance_table.read(cx).delegate();
        let is_empty = delegate.has_no_instances();
        let no_results = delegate.has_no_visible_instances() && delegate.is_filter_active();

        if is_empty {
            let logo_scale = animation::animated_logo_scale(window, cx);
            let theme = cx.theme();
            let logo_size = px(64.0 * logo_scale);
            return v_flex()
                .size_full()
                .p_8()
                .gap_4()
                .justify_center()
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
                        .text_xl()
                        .font_family(SharedString::new_static(MINECRAFT_FONT))
                        .text_color(theme.sidebar_primary)
                        .child(t::instance::welcome::title()),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .max_w_96()
                        .text_center()
                        .child(t::instance::welcome::subtitle()),
                )
                .child(
                    Button::new("welcome_create")
                        .success()
                        .icon(QuartzIcon::Plus)
                        .label(t::instance::create())
                        .on_click(cx.listener(|this, _, window, cx| {
                            modals::create_instance::open_create_instance(
                                this.metadata.clone(),
                                this.instances.clone(),
                                this.backend_handle.clone(),
                                window,
                                cx,
                            );
                        })),
                )
                .into_any_element();
        }

        if no_results {
            return v_flex()
                .size_full()
                .p_8()
                .justify_center()
                .items_center()
                .text_color(cx.theme().muted_foreground)
                .child(t::instance::search_no_results())
                .into_any_element();
        }

        match InterfaceConfig::get(cx).instances_view_mode {
            InstancesViewMode::Cards => {
                let cards = self.instance_table.update(cx, |table, cx| {
                    let rows = table.delegate().visible_count();
                    (0..rows)
                        .map(|i| table.delegate().render_card(i, window, cx))
                        .collect::<Vec<_>>()
                });

                let size = Size::new(
                    gpui::AvailableSpace::MinContent,
                    gpui::AvailableSpace::MinContent
                );

                div().p_4().child(ResponsiveGrid::new(size).size_full().gap_4().children(cards)).into_any_element()
            },
            InstancesViewMode::List => {
                DataTable::new(&self.instance_table).bordered(false).into_any_element()
            },
        }
    }
}

#[derive(Default)]
pub struct VersionList {
    pub versions: Vec<SharedString>,
    pub matched_versions: Vec<SharedString>,
}

impl SelectDelegate for VersionList {
    type Item = SharedString;

    fn items_count(&self, _section: usize) -> usize {
        self.matched_versions.len()
    }

    fn item(&self, ix: IndexPath) -> Option<&Self::Item> {
        self.matched_versions.get(ix.row)
    }

    fn position<V>(&self, value: &V) -> Option<IndexPath>
    where
        Self::Item: gpui_component::select::SelectItem<Value = V>,
        V: PartialEq,
    {
        for (ix, item) in self.matched_versions.iter().enumerate() {
            if item.value() == value {
                return Some(IndexPath::default().row(ix));
            }
        }

        None
    }

    fn perform_search(&mut self, query: &str, _window: &mut Window, _: &mut Context<SelectState<Self>>) -> Task<()> {
        let lower_query = query.to_lowercase();

        self.matched_versions = self
            .versions
            .iter()
            .filter(|item| item.to_lowercase().starts_with(&lower_query))
            .cloned()
            .collect();

        Task::ready(())
    }
}
