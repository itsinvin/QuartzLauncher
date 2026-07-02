use bridge::instance::ContentFolder;
use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme as _, Disableable, Sizable, StyledExt, button::Button, h_flex, input::{InputEvent, InputState, NumberInput}, spinner::Spinner, v_flex};
use prediction::{detect_hardware, predict_performance, HardwareProfile, WorkloadProfile};
use schema::instance::InstanceMemoryConfiguration;

use crate::{
    entity::instance::InstanceEntry,
    icon::QuartzIcon,
    pages::performance_page::{hardware_panel, prediction_panel, section_title},
};

const OPTIMIZATION_MOD_IDS: &[&str] = &[
    "sodium", "lithium", "ferrite-core", "ferritecore", "starlight", "modernfix", "embeddium",
    "rubidium", "c2me-fabric", "lazydfu", "krypton", "entityculling",
];

const HEAVY_MOD_IDS: &[&str] = &[
    "create", "applied-energistics-2", "ae2", "twilightforest", "botania", "mekanism",
    "immersive-engineering", "ad-astra", "better-end", "better-nether",
];

pub struct InstancePerformanceSubpage {
    instance: Entity<InstanceEntry>,
    hardware: Option<HardwareProfile>,
    detecting: bool,
    render_distance_input: Entity<InputState>,
    _detect_task: Task<()>,
    _input_subscription: Subscription,
}

impl InstancePerformanceSubpage {
    pub fn new(instance: &Entity<InstanceEntry>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let render_distance_input = cx.new(|cx| {
            InputState::new(window, cx).default_value("12".to_string())
        });

        let _input_subscription = cx.subscribe_in(&render_distance_input, window, |_, _, _: &InputEvent, _, cx| {
            cx.notify();
        });

        let mut page = Self {
            instance: instance.clone(),
            hardware: None,
            detecting: true,
            render_distance_input,
            _detect_task: Task::ready(()),
            _input_subscription,
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
                cx.notify();
            });
        });
    }

    fn workload(&self, cx: &App) -> WorkloadProfile {
        let instance = self.instance.read(cx);
        let config = &instance.configuration;
        let loader = config.loader.pretty_name().to_ascii_lowercase();
        let mc_version = config.minecraft_version.to_string();

        let memory = config.memory.unwrap_or_default();
        let allocated_ram_mb = if memory.enabled { memory.max } else { InstanceMemoryConfiguration::DEFAULT_MAX };

        let mods = instance.content[ContentFolder::Mods].read(cx);
        let mut mod_count = 0u32;
        let mut optimization_mods = false;
        let mut heavy_mods = false;

        for summary in mods.iter().filter(|m| m.enabled) {
            if let Some(files) = summary.content_summary.extra.modpack_files() {
                mod_count += files.iter().filter(|file| {
                    file.path.as_str().starts_with("mods/")
                }).count() as u32;
            } else {
                mod_count += 1;
            }

            if let Some(id) = summary.content_summary.id.as_deref() {
                let id_lower = id.to_ascii_lowercase();
                optimization_mods |= OPTIMIZATION_MOD_IDS.iter().any(|opt| id_lower.contains(opt));
                heavy_mods |= HEAVY_MOD_IDS.iter().any(|heavy| id_lower.contains(heavy));
            }
        }

        if mod_count > 120 {
            heavy_mods = true;
        }

        let shaders = instance.content[ContentFolder::Shaders].read(cx).iter().any(|s| s.enabled)
            || config.show_shader_tab;

        let render_distance = self.render_distance_input.read(cx).value().parse::<u32>().unwrap_or(12).clamp(2, 32);

        WorkloadProfile {
            name: instance.name.to_string(),
            mc_version,
            loader,
            mod_count,
            allocated_ram_mb,
            render_distance,
            shaders,
            optimization_mods,
            heavy_mods,
        }
    }

    fn prediction(&self, cx: &App) -> Option<prediction::PerformancePrediction> {
        let hardware = self.hardware.as_ref()?;
        Some(predict_performance(hardware, &self.workload(cx)))
    }
}

impl Render for InstancePerformanceSubpage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.instance.read(cx).content_states.observe(ContentFolder::Mods);
        self.instance.read(cx).content_states.observe(ContentFolder::Shaders);

        let theme = cx.theme();
        let workload = self.workload(cx);

        let mut root = v_flex().size_full().p_4().gap_4()
            .child(
                h_flex().justify_between().items_center()
                    .child(div().text_sm().text_color(theme.muted_foreground)
                        .child(t::instance::performance::subtitle()))
                    .child(Button::new("refresh_hardware").outline().icon(QuartzIcon::RefreshCcw)
                        .label(t::tools::performance::refresh())
                        .disabled(self.detecting)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.refresh_hardware(cx);
                        }))),
            );

        if self.detecting {
            return root.child(h_flex().gap_2().items_center()
                .child(Spinner::new())
                .child(t::tools::performance::detecting())).into_any_element();
        }

        if let Some(hardware) = &self.hardware {
            root = root.child(section_title(t::tools::performance::hardware()))
                .child(hardware_panel(hardware, cx));
        }

        root = root.child(section_title(t::instance::performance::workload()))
            .child(
                v_flex().gap_3().p_3().rounded_lg().border_1().border_color(theme.border)
                    .child(workload_row(t::tools::performance::mod_count(), workload.mod_count.to_string()))
                    .child(workload_row(t::instance::version(), workload.mc_version.clone()))
                    .child(workload_row(t::instance::performance::loader(), workload.loader.clone()))
                    .child(workload_row(t::tools::performance::allocated_ram(), format!("{} MiB", workload.allocated_ram_mb)))
                    .child(h_flex().gap_4().items_end()
                        .child(crate::labelled(
                            t::tools::performance::render_distance(),
                            NumberInput::new(&self.render_distance_input).small(),
                        )))
                    .child(workload_flags(workload, cx)),
            );

        if let Some(prediction) = self.prediction(cx) {
            root = root.child(section_title(t::tools::performance::predict()))
                .child(prediction_panel(&prediction, cx));
        }

        root.into_any_element()
    }
}

fn workload_row(label: impl Into<SharedString>, value: impl Into<SharedString>) -> impl IntoElement {
    h_flex().gap_2()
        .child(div().w_40().text_sm().font_medium().child(label.into()))
        .child(div().text_sm().child(value.into()))
}

fn workload_flags(workload: WorkloadProfile, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    h_flex().gap_3().flex_wrap().text_sm().text_color(theme.muted_foreground)
        .child(flag_text(if workload.shaders { t::tools::performance::shaders() } else { t::instance::performance::no_shaders() }))
        .child("•")
        .child(flag_text(if workload.optimization_mods { t::instance::performance::optimization_detected() } else { t::instance::performance::no_optimization_mods() }))
        .child("•")
        .child(flag_text(if workload.heavy_mods { t::instance::performance::heavy_modpack() } else { t::instance::performance::light_medium_load() }))
}

fn flag_text(text: impl Into<SharedString>) -> SharedString {
    text.into()
}
