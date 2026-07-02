use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme as _, Disableable, Sizable, StyledExt, button::Button, checkbox::Checkbox,
    h_flex, input::{InputEvent, InputState, NumberInput}, spinner::Spinner, v_flex,
};
use prediction::{
    detect_hardware, predict_performance, Bottleneck, HardwareProfile, PerfRating, PerformancePrediction,
    WorkloadProfile,
};

use crate::{icon::QuartzIcon, pages::page::Page};

pub struct PerformancePage {
    hardware: Option<HardwareProfile>,
    prediction: Option<PerformancePrediction>,
    detecting: bool,
    mod_count_input: Entity<InputState>,
    ram_input: Entity<InputState>,
    render_distance_input: Entity<InputState>,
    shaders: bool,
    optimization_mods: bool,
    heavy_mods: bool,
    _detect_task: Task<()>,
    _input_subscriptions: [Subscription; 3],
}

impl PerformancePage {
    pub fn new(_data: &crate::entity::DataEntities, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mod_count_input = cx.new(|cx| {
            InputState::new(window, cx).default_value("80".to_string())
        });
        let ram_input = cx.new(|cx| {
            InputState::new(window, cx).default_value("4096".to_string())
        });
        let render_distance_input = cx.new(|cx| {
            InputState::new(window, cx).default_value("12".to_string())
        });

        let _input_subscriptions = [
            cx.subscribe_in(&mod_count_input, window, |this, _, _: &InputEvent, _, cx| {
                this.update_prediction(cx);
            }),
            cx.subscribe_in(&ram_input, window, |this, _, _: &InputEvent, _, cx| {
                this.update_prediction(cx);
            }),
            cx.subscribe_in(&render_distance_input, window, |this, _, _: &InputEvent, _, cx| {
                this.update_prediction(cx);
            }),
        ];

        let mut page = Self {
            hardware: None,
            prediction: None,
            detecting: true,
            mod_count_input,
            ram_input,
            render_distance_input,
            shaders: false,
            optimization_mods: true,
            heavy_mods: false,
            _detect_task: Task::ready(()),
            _input_subscriptions,
        };

        page.refresh_hardware(cx);
        page
    }

    fn refresh_hardware(&mut self, cx: &mut Context<Self>) {
        self.detecting = true;
        self._detect_task = cx.spawn(async move |page, cx| {
            let hardware = detect_hardware();
            let _ = page.update(cx, |page, cx| {
                page.hardware = Some(hardware);
                page.detecting = false;
                page.update_prediction(cx);
                cx.notify();
            });
        });
    }

    fn workload(&self, cx: &App) -> WorkloadProfile {
        let parse_u32 = |state: &Entity<InputState>, default: u32| {
            state.read(cx).value().parse::<u32>().unwrap_or(default)
        };

        WorkloadProfile {
            name: "Estimate".to_string(),
            mc_version: "1.21".to_string(),
            loader: "fabric".to_string(),
            mod_count: parse_u32(&self.mod_count_input, 80),
            allocated_ram_mb: parse_u32(&self.ram_input, 4096),
            render_distance: parse_u32(&self.render_distance_input, 12).clamp(2, 32),
            shaders: self.shaders,
            optimization_mods: self.optimization_mods,
            heavy_mods: self.heavy_mods,
        }
    }

    fn update_prediction(&mut self, cx: &mut Context<Self>) {
        let Some(hardware) = self.hardware.clone() else {
            self.prediction = None;
            return;
        };
        self.prediction = Some(predict_performance(&hardware, &self.workload(cx)));
        cx.notify();
    }

    fn format_rating(rating: &PerfRating) -> &'static str {
        match rating {
            PerfRating::Unplayable => "Unplayable",
            PerfRating::Choppy => "Choppy",
            PerfRating::Playable => "Playable",
            PerfRating::Smooth => "Smooth",
            PerfRating::Excellent => "Excellent",
        }
    }

    fn format_bottleneck(bottleneck: &Bottleneck) -> &'static str {
        match bottleneck {
            Bottleneck::Cpu => "CPU",
            Bottleneck::Gpu => "GPU",
            Bottleneck::Ram => "RAM",
            Bottleneck::Storage => "Storage",
            Bottleneck::Balanced => "Balanced",
        }
    }

    fn rating_color(rating: &PerfRating, cx: &App) -> Hsla {
        let theme = cx.theme();
        match rating {
            PerfRating::Unplayable | PerfRating::Choppy => theme.danger,
            PerfRating::Playable => theme.warning,
            PerfRating::Smooth | PerfRating::Excellent => theme.success,
        }
    }
}

impl Page for PerformancePage {
    fn controls(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Button::new("refresh_hardware")
            .outline()
            .icon(QuartzIcon::RefreshCcw)
            .label(t::tools::performance::refresh())
            .disabled(self.detecting)
            .on_click(cx.listener(|this, _, _, cx| {
                this.refresh_hardware(cx);
            }))
    }

    fn scrollable(&self, _cx: &App) -> bool {
        true
    }
}

impl Render for PerformancePage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let subtitle = t::tools::performance::subtitle();
        let mut root = v_flex().size_full().p_4().gap_4()
            .child(div().text_sm().text_color(cx.theme().muted_foreground).child(subtitle));

        if self.detecting {
            return root.child(h_flex().gap_2().items_center()
                .child(Spinner::new())
                .child(t::tools::performance::detecting())).into_any_element();
        }

        if let Some(hardware) = &self.hardware {
            root = root.child(section_title(t::tools::performance::hardware()))
                .child(hardware_panel(hardware, cx));
        }

        root = root
            .child(section_title(t::tools::performance::workload()))
            .child(workload_panel(self, cx));

        if let Some(prediction) = &self.prediction {
            root = root
                .child(section_title(t::tools::performance::predict()))
                .child(prediction_panel(prediction, cx));
        }

        root.into_any_element()
    }
}

pub(crate) fn section_title(title: impl Into<SharedString>) -> Div {
    div().text_lg().font_semibold().child(title.into())
}

pub(crate) fn hardware_panel(hardware: &HardwareProfile, cx: &App) -> Div {
    let ram_gb = hardware.total_ram_mb as f64 / 1024.0;
    v_flex().gap_2().p_3().rounded_lg().border_1().border_color(cx.theme().border)
        .child(info_row(t::tools::performance::cpu(), &hardware.cpu.brand))
        .child(info_row(t::tools::performance::gpu(), &hardware.gpu.model))
        .child(info_row(
            t::tools::performance::ram(),
            format!("{ram_gb:.1} GiB"),
        ))
}

fn info_row(label: impl Into<SharedString>, value: impl Into<SharedString>) -> Div {
    h_flex().gap_2().child(div().w_32().text_sm().font_medium().child(label.into()))
        .child(div().text_sm().child(value.into()))
}

fn workload_panel(page: &mut PerformancePage, cx: &mut Context<PerformancePage>) -> Div {
    let workload_options = v_flex().gap_2()
        .child(Checkbox::new("shaders")
            .label(t::tools::performance::shaders())
            .checked(page.shaders)
            .on_click(cx.listener(|page, value, _, cx| {
                page.shaders = *value;
                page.update_prediction(cx);
            })))
        .child(Checkbox::new("optimization_mods")
            .label(t::tools::performance::optimization_mods())
            .checked(page.optimization_mods)
            .on_click(cx.listener(|page, value, _, cx| {
                page.optimization_mods = *value;
                page.update_prediction(cx);
            })))
        .child(Checkbox::new("heavy_mods")
            .label(t::tools::performance::heavy_mods())
            .checked(page.heavy_mods)
            .on_click(cx.listener(|page, value, _, cx| {
                page.heavy_mods = *value;
                page.update_prediction(cx);
            })));

    v_flex().gap_3().p_3().rounded_lg().border_1().border_color(cx.theme().border)
        .child(h_flex().gap_4().flex_wrap()
            .child(crate::labelled(
                t::tools::performance::mod_count(),
                NumberInput::new(&page.mod_count_input).small(),
            ))
            .child(crate::labelled(
                t::tools::performance::allocated_ram(),
                NumberInput::new(&page.ram_input).small().suffix("MiB"),
            ))
            .child(crate::labelled(
                t::tools::performance::render_distance(),
                NumberInput::new(&page.render_distance_input).small(),
            )))
        .child(workload_options)
}

pub(crate) fn prediction_panel(prediction: &PerformancePrediction, cx: &App) -> Div {
    let rating_label = PerformancePage::format_rating(&prediction.rating);
    let rating_color = PerformancePage::rating_color(&prediction.rating, cx);
    let bottleneck = PerformancePage::format_bottleneck(&prediction.bottleneck);

    let mut panel = v_flex().gap_3().p_3().rounded_lg().border_1().border_color(cx.theme().border)
        .child(h_flex().gap_6().flex_wrap()
            .child(stat_block(
                t::tools::performance::avg_fps(),
                format!("{}", prediction.avg_fps),
                cx.theme().foreground,
            ))
            .child(stat_block(
                t::tools::performance::fps_range(),
                format!("{}–{}", prediction.fps_range[0], prediction.fps_range[1]),
                cx.theme().foreground,
            ))
            .child(stat_block(
                t::tools::performance::rating(),
                rating_label.to_string(),
                rating_color,
            ))
            .child(stat_block(
                t::tools::performance::bottleneck(),
                bottleneck.to_string(),
                cx.theme().foreground,
            )));

    if !prediction.recommendations.is_empty() {
        let mut recs = v_flex().gap_2()
            .child(div().text_sm().font_medium().child(t::tools::performance::recommendations()));
        for rec in &prediction.recommendations {
            recs = recs.child(v_flex().gap_0p5()
                .child(div().text_sm().font_medium().child(rec.title.clone()))
                .child(div().text_sm().text_color(cx.theme().muted_foreground).child(rec.detail.clone())));
        }
        panel = panel.child(recs);
    }

    panel
}

fn stat_block(label: impl Into<SharedString>, value: impl Into<SharedString>, color: Hsla) -> Div {
    v_flex().gap_0p5()
        .child(div().text_xs().text_color(color).child(label.into()))
        .child(div().text_xl().font_semibold().text_color(color).child(value.into()))
}
