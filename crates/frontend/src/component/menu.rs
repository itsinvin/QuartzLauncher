use std::rc::Rc;

use gpui::{prelude::*, transparent_black, App, ClickEvent, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div};
use gpui_component::{ActiveTheme, StyledExt, h_flex, v_flex};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MenuIndicator {
    Running,
}

#[derive(IntoElement)]
pub struct MenuGroup {
    title: SharedString,
    children: Vec<MenuGroupItem>,
}

impl MenuGroup {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            children: Vec::new(),
        }
    }

    pub fn child(mut self, child: MenuGroupItem) -> Self {
        self.children.push(child);
        self
    }
}

impl RenderOnce for MenuGroup {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
        let title = div()
            .px_2()
            .pt_1()
            .text_xs()
            .font_medium()
            .text_color(cx.theme().sidebar_foreground.opacity(0.55))
            .child(self.title);

        v_flex().gap_0p5().child(title).children(self.children)
    }
}

#[derive(IntoElement)]
pub struct MenuGroupItem {
    title: SharedString,
    active: bool,
    indicator: Option<MenuIndicator>,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
}

impl MenuGroupItem {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            active: false,
            indicator: None,
            on_click: None,
        }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn indicator(mut self, indicator: MenuIndicator) -> Self {
        self.indicator = Some(indicator);
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for MenuGroupItem {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
        let theme = cx.theme();
        let label = h_flex()
            .gap_2()
            .items_center()
            .when_some(self.indicator, |this, indicator| {
                this.child(match indicator {
                    MenuIndicator::Running => div()
                        .size_2()
                        .rounded_full()
                        .bg(theme.success)
                        .shadow_sm(),
                })
            })
            .child(self.title);

        let mut item = div()
            .id(self.title.clone())
            .px_2()
            .py_1()
            .text_sm()
            .child(label)
            .rounded(theme.radius)
            .when_some(self.on_click, |this, on_click| {
                this.on_click(move |event, window, cx| {
                    (on_click)(event, window, cx);
                })
            });

        if self.active {
            item = item
                .font_medium()
                .bg(theme.sidebar_accent)
                .text_color(theme.sidebar_accent_foreground)
                .border_l_2()
                .border_color(theme.sidebar_primary);
        } else {
            item = item
                .border_l_2()
                .border_color(transparent_black())
                .hover(|this| {
                    this.bg(theme.sidebar_accent.opacity(0.9))
                        .text_color(theme.sidebar_accent_foreground)
                        .border_color(theme.sidebar_primary.opacity(0.45))
                });
        }

        item
    }
}
