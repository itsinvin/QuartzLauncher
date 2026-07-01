use bridge::{handle::BackendHandle, instance::InstanceStatus, message::MessageToBackend};
use gpui::{prelude::*, *};
use gpui_component::{
    button::{Button, ButtonVariants}, h_flex, table::{Column, ColumnSort, TableDelegate, TableState}, v_flex, ActiveTheme, Icon, Sizable
};

use crate::{component::animation::format_last_played, entity::{
        instance::{InstanceAddedEvent, InstanceEntry, InstanceModifiedEvent, InstanceRemovedEvent}, DataEntities
    }, png_render_cache, root, ui,
};

pub struct InstanceList {
    columns: Vec<Column>,
    items: Vec<InstanceEntry>,
    visible_indices: Vec<usize>,
    filter_query: SharedString,
    backend_handle: BackendHandle,
    _instance_added_subscription: Subscription,
    _instance_removed_subscription: Subscription,
    _instance_modified_subscription: Subscription,
}

impl InstanceList {
    pub fn set_filter(&mut self, query: SharedString) {
        if self.filter_query == query {
            return;
        }
        self.filter_query = query;
        self.rebuild_visible_indices();
    }

    fn rebuild_visible_indices(&mut self) {
        if self.filter_query.is_empty() {
            self.visible_indices = (0..self.items.len()).collect();
            return;
        }

        let query = self.filter_query.to_lowercase();
        self.visible_indices = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                item.name.to_lowercase().contains(&query)
                    || item.configuration.minecraft_version.as_str().to_lowercase().contains(&query)
                    || item.configuration.loader.pretty_name().to_lowercase().contains(&query)
            })
            .map(|(index, _)| index)
            .collect();
    }

    fn item_at(&self, visible_index: usize) -> Option<&InstanceEntry> {
        self.visible_indices
            .get(visible_index)
            .and_then(|&index| self.items.get(index))
    }

    pub fn has_no_instances(&self) -> bool {
        self.items.is_empty()
    }

    pub fn has_no_visible_instances(&self) -> bool {
        self.visible_indices.is_empty()
    }

    pub fn is_filter_active(&self) -> bool {
        !self.filter_query.is_empty()
    }

    pub fn visible_count(&self) -> usize {
        self.visible_indices.len()
    }

    fn sort_by_last_played(items: &mut [InstanceEntry]) {
        items.sort_by_key(|item| -(item.playtime.last_played_unix_ms.unwrap_or(0)));
    }
    pub fn create_table(data: &DataEntities, window: &mut Window, cx: &mut App) -> Entity<TableState<Self>> {
        let instances = data.instances.clone();
        let mut items = instances.read(cx).entries.values().map(|i| i.read(cx).clone()).collect::<Vec<_>>();
        Self::sort_by_last_played(&mut items);
        cx.new(|cx| {
            let _instance_added_subscription = cx.subscribe::<_, InstanceAddedEvent>(&instances, |table: &mut TableState<InstanceList>, _, event, cx| {
                table.delegate_mut().items.insert(0, event.instance.clone());
                InstanceList::sort_by_last_played(&mut table.delegate_mut().items);
                table.delegate_mut().rebuild_visible_indices();
                cx.notify();
            });
            let _instance_removed_subscription = cx.subscribe::<_, InstanceRemovedEvent>(&instances, |table, _, event, cx| {
                table.delegate_mut().items.retain(|instance| {
                    instance.id != event.id
                });
                table.delegate_mut().rebuild_visible_indices();
                cx.notify();
            });
            let _instance_modified_subscription = cx.subscribe::<_, InstanceModifiedEvent>(&instances, |table, _, event, cx| {
                if let Some(entry) = table.delegate_mut().items.iter_mut().find(|entry| entry.id == event.instance.id) {
                    *entry = event.instance.clone();
                    InstanceList::sort_by_last_played(&mut table.delegate_mut().items);
                    table.delegate_mut().rebuild_visible_indices();
                    cx.notify();
                }
            });
            let visible_indices: Vec<usize> = (0..items.len()).collect();
            let instance_list = Self {
                columns: vec![
                    Column::new("controls", "")
                        .width(150.)
                        .fixed_left()
                        .movable(false)
                        .resizable(false),
                    Column::new("name", t::instance::name())
                        .width(150.)
                        .fixed_left()
                        .sortable()
                        .resizable(true),
                    Column::new("version", t::instance::version())
                        .width(150.)
                        .fixed_left()
                        .sortable()
                        .resizable(true),
                    Column::new("loader", t::instance::modloader())
                        .width(150.)
                        .fixed_left()
                        .resizable(true),
                ],
                items,
                visible_indices,
                filter_query: SharedString::default(),
                backend_handle: data.backend_handle.clone(),
                _instance_added_subscription,
                _instance_removed_subscription,
                _instance_modified_subscription,
            };
            TableState::new(instance_list, window, cx)
        })
    }

    pub fn render_card(&self, index: usize, cx: &mut App) -> Div {
        let Some(item) = self.item_at(index) else {
            return div();
        };
        let loader_and_version = format!(
            "{} {}",
            item.configuration.loader.pretty_name(),
            item.configuration.minecraft_version.as_str(),
        );
        let last_played = format_last_played(item.playtime.last_played_unix_ms);
        let real_index = self.visible_indices[index];

        let icon = if let Some(icon) = item.icon.clone() {
            let transform = png_render_cache::ImageTransformation::Resize { width: 64, height: 64 };
            png_render_cache::render_with_transform(icon, transform, cx)
                .rounded(cx.theme().radius).size_16().min_w_16().min_h_16().into_any_element()
        } else {
            let icon_path = item.configuration.instance_fallback_icon
                .map(|s| s.as_str())
                .unwrap_or("icons/box.svg");
            Icon::default().path(icon_path).size_16().min_w_16().min_h_16().into_any_element()
        };

        let play_button = render_play_button(item, real_index, self.backend_handle.clone());

        let theme = cx.theme();
        v_flex()
            .flex_1()
            .p_2()
            .gap_2()
            .w_full()
            .min_w_64()
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius_lg)
            .hover(|this| this.border_color(theme.ring.opacity(0.6)))
            .child(h_flex()
                .w_full()
                .gap_2()
                .child(icon)
                .child(v_flex()
                    .truncate()
                    .w_full()
                    .child(item.name.clone())
                    .child(loader_and_version)
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(last_played),
                    )
                )
            ).child(h_flex()
                .gap_2()
                .child(play_button.flex_1().small())
                .child(Button::new(("view", real_index)).flex_1().small().info().label(t::instance::view()).on_click({
                    let name = item.name.clone();
                    move |_, window, cx| {
                        root::switch_page(ui::PageType::InstancePage { name: name.clone() },
                            &[ui::PageType::Instances], window, cx);
                    }
                })))

    }
}

impl TableDelegate for InstanceList {
    fn columns_count(&self, _cx: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _cx: &App) -> usize {
        self.visible_indices.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> gpui_component::table::Column {
        self.columns[col_ix].clone()
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: gpui_component::table::ColumnSort,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) {
        if let Some(col) = self.columns.get_mut(col_ix) {
            match col.key.as_ref() {
                "name" => self.items.sort_by(|a, b| match sort {
                    ColumnSort::Descending => lexical_sort::natural_lexical_cmp(&a.name, &b.name).reverse(),
                    _ => lexical_sort::natural_lexical_cmp(&a.name, &b.name),
                }),
                "version" => self.items.sort_by(|a, b| match sort {
                    ColumnSort::Descending => lexical_sort::natural_lexical_cmp(&a.configuration.minecraft_version, &b.configuration.minecraft_version).reverse(),
                    _ => lexical_sort::natural_lexical_cmp(&a.configuration.minecraft_version, &b.configuration.minecraft_version),
                }),
                _ => {},
            }
        }
    }

    fn render_td(&mut self, row_ix: usize, col_ix: usize, _window: &mut Window, _cx: &mut Context<TableState<Self>>) -> impl IntoElement {
        let Some(item) = self.item_at(row_ix) else {
            return t::common::unknown().into_any_element();
        };
        let real_index = self.visible_indices[row_ix];
        if let Some(col) = self.columns.get(col_ix) {
            match col.key.as_ref() {
                "name" => item.name.clone().into_any_element(),
                "version" => item.configuration.minecraft_version.as_str().into_any_element(),
                "controls" => {
                    let play_button = render_play_button(item, real_index, self.backend_handle.clone());

                    h_flex()
                        .size_full()
                        .gap_2()
                        .border_r_4()
                        .child(play_button.w_1_2().small())
                        .child(Button::new("view").w_1_2().small().info().label(t::instance::view()).on_click({
                            let name = item.name.clone();
                            move |_, window, cx| {
                                root::switch_page(ui::PageType::InstancePage { name: name.clone() },
                                    &[ui::PageType::Instances], window, cx);
                            }
                        }))
                        .into_any_element()
                },
                "loader" => item.configuration.loader.pretty_name().into_any_element(),
                _ => t::common::unknown().into_any_element(),
            }
        } else {
            t::common::unknown().into_any_element()
        }
    }
}

fn render_play_button(item: &InstanceEntry, index: usize, backend_handle: BackendHandle) -> Button {
    let name = item.name.clone();
    let id = item.id;
    match item.status {
        InstanceStatus::NotRunning => {
            Button::new(("start_instance", index))
                .success()
                .label(t::instance::start::label())
                .on_click(
                move |_, window, cx| {
                    root::start_instance(id, name.clone(), None, &backend_handle, window, cx);
                },
            )
        },
        InstanceStatus::Launching => {
            Button::new(("launching", index))
                .warning()
                .label("...")
        },
        InstanceStatus::Stopping => {
            Button::new(("launching", index))
                .danger()
                .label("...")
        },
        InstanceStatus::Running => {
            Button::new(("kill_instance", index))
                .danger()
                .label(t::instance::kill())
                .on_click({
                    let backend_handle = backend_handle.clone();
                    move |_, _, _| {
                        backend_handle.send(MessageToBackend::KillInstance { id });
                    }
                })
        },
    }
}
