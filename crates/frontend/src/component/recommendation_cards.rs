use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme as _, Icon, Sizable, StyledExt,
    button::{Button, ButtonVariants},
    h_flex, v_flex,
};
use schema::modrinth::ModrinthHit;

use crate::{
    component::responsive_grid::ResponsiveGrid,
    icon::QuartzIcon,
    root,
    ui::PageType,
};

pub struct RecommendationCard {
    pub title: SharedString,
    pub subtitle: SharedString,
    pub thumbnail: Option<SharedString>,
    pub page: PageType,
}

impl RecommendationCard {
    pub fn from_modrinth_hit(hit: &ModrinthHit, install_for: Option<SharedString>, path: &[PageType]) -> Self {
        let project_id = hit.project_id.to_string();
        let project_title = hit.title.as_deref().unwrap_or("").to_string();
        Self {
            title: project_title.clone().into(),
            subtitle: hit.author.to_string().into(),
            thumbnail: hit.icon_url.clone().map(|url| SharedString::from(url.to_string())),
            page: PageType::ModrinthProject {
                project_id: project_id.into(),
                project_title: project_title.into(),
                install_for,
            },
        }
    }

    pub fn render(self, section_id: SharedString, index: usize, path: &[PageType], cx: &mut App) -> AnyElement {
        let theme = cx.theme();
        let card_muted = theme.muted;
        let card_border = theme.border;
        let card_radius = theme.radius;
        let card_secondary = theme.secondary;
        let card_list_hover = theme.list_hover;
        let card_sidebar_primary = theme.sidebar_primary;
        let card_muted_foreground = theme.muted_foreground;
        let page = self.page;
        let path = path.to_vec();

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
                root::switch_page(page.clone(), &path, window, cx);
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

pub fn recommendation_section(
    title: impl Into<SharedString>,
    empty_message: impl Into<SharedString>,
    browse_label: impl Into<SharedString>,
    section_id: SharedString,
    action_id: SharedString,
    cards: Vec<RecommendationCard>,
    path: &[PageType],
    on_browse: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    loading_message: Option<SharedString>,
    error_message: Option<SharedString>,
    cx: &mut App,
) -> AnyElement {
    let card_elements = cards
        .into_iter()
        .enumerate()
        .map(|(index, card)| card.render(section_id.clone(), index, path, cx))
        .collect::<Vec<_>>();

    let theme = cx.theme();
    let empty_radius = theme.radius;
    let empty_border = theme.border;
    let empty_muted = theme.muted_foreground;
    let has_cards = !card_elements.is_empty();
    let title = title.into();
    let empty_message = empty_message.into();
    let browse_label = browse_label.into();

    let mut section = v_flex()
        .w_full()
        .gap_3()
        .child(
            h_flex()
                .justify_between()
                .items_center()
                .child(div().text_lg().font_medium().child(title))
                .child(
                    Button::new(action_id)
                        .compact()
                        .small()
                        .info()
                        .label(browse_label)
                        .on_click(on_browse),
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
        .into_any_element();

    if let Some(message) = loading_message {
        section = v_flex()
            .w_full()
            .gap_3()
            .child(section)
            .child(div().text_sm().text_color(theme.muted_foreground).child(message))
            .into_any_element();
    } else if let Some(error) = error_message {
        section = v_flex()
            .w_full()
            .gap_3()
            .child(section)
            .child(div().text_sm().text_color(theme.muted_foreground).child(error))
            .into_any_element();
    }

    section
}
