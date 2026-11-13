//! The settings dock: an always-visible right-side rail (the
//! Photoshop/Blender model). Sections expand independently and stay open
//! (the rail scrolls on overflow); the `menu` script action opens a
//! section exclusively so tests stay deterministic.

use bevy::prelude::*;
use bevy_egui::egui;

use crate::Link;
use crate::cursors::CursorState;
use crate::derived::{FftState, MathState, MeasureState, PfState};
use crate::gpu::Phosphor;

use super::layout::{Layout, Roi};
use super::{
    dialog_acquire, dialog_channel, dialog_cursor, dialog_decode, dialog_display,
    dialog_horizontal, dialog_math, dialog_measure, dialog_record, dialog_trigger, dialog_utility,
};

use crate::record::Recorder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    More,
    Annotations,
    Automotive,
    Channel(usize),
    Horizontal,
    Trigger,
    Acquire,
    Display,
    Measure,
    Math,
    Cursor,
    Utility,
    Record,
    Decode,
}

impl Menu {
    pub fn title(self) -> &'static str {
        match self {
            Menu::More => "More... (Todas las Funciones)",
            Menu::Annotations => "Anotaciones y Gráfica",
            Menu::Automotive => "🚗 Diagnóstico Automotriz",
            Menu::Channel(ch) => match ch {
                0 => "Channel 1 (Canal A)",
                1 => "Channel 2 (Canal B)",
                _ => "Channel",
            },
            Menu::Horizontal => "Horizontal (Base de Tiempo)",
            Menu::Trigger => "Trigger (Disparo)",
            Menu::Acquire => "Acquire (Adquisición)",
            Menu::Display => "Display & Referencias",
            Menu::Measure => "Measurements (Mediciones)",
            Menu::Math => "Math Channels (Matemáticas)",
            Menu::Cursor => "Rulers (Reglas)",
            Menu::Utility => "Masks & Actions (Máscaras)",
            Menu::Record => "Buffer & Export (Grabación)",
            Menu::Decode => "🔢 Serial Decode (CAN/LIN)",
        }
    }

    /// Stable name: the `layout` dump and the `dock` action share it.
    pub fn name(self) -> &'static str {
        match self {
            Menu::More => "more",
            Menu::Annotations => "annotations",
            Menu::Automotive => "automotive",
            Menu::Channel(0) => "channel0",
            Menu::Channel(1) => "channel1",
            Menu::Channel(_) => "channel",
            Menu::Horizontal => "horizontal",
            Menu::Trigger => "trigger",
            Menu::Acquire => "acquire",
            Menu::Display => "display",
            Menu::Measure => "measure",
            Menu::Math => "math",
            Menu::Cursor => "cursor",
            Menu::Utility => "utility",
            Menu::Record => "record",
            Menu::Decode => "decode",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        Some(match s {
            "more" => Menu::More,
            "annotations" => Menu::Annotations,
            "automotive" => Menu::Automotive,
            "channel0" => Menu::Channel(0),
            "channel1" => Menu::Channel(1),
            "horizontal" => Menu::Horizontal,
            "trigger" => Menu::Trigger,
            "acquire" => Menu::Acquire,
            "display" => Menu::Display,
            "measure" => Menu::Measure,
            "math" => Menu::Math,
            "cursor" => Menu::Cursor,
            "utility" => Menu::Utility,
            "record" => Menu::Record,
            "decode" => Menu::Decode,
            _ => return None,
        })
    }
}

/// Which dock sections are expanded. Sections are independent — leave as
/// many open as you like; the rail scrolls.
#[derive(Resource)]
pub struct MenuState {
    open: Vec<Menu>,
    /// Section to scroll into view on the next frame. Opening a section from
    /// the front panel or the View menu is useless if it lands off-screen in
    /// the scrolled rail.
    focus: Option<Menu>,
    pub collapsed: bool,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            open: Vec::new(),
            focus: None,
            collapsed: true,
        }
    }
}

impl MenuState {
    pub fn is_collapsed(&self) -> bool {
        self.collapsed || self.open.is_empty()
    }

    pub fn set_collapsed(&mut self, val: bool) {
        self.collapsed = val;
        if val {
            self.open.clear();
        }
    }

    pub fn toggle_collapsed(&mut self) {
        self.collapsed = !self.collapsed;
        if !self.collapsed && self.open.is_empty() {
            self.open.push(Menu::Cursor);
        }
    }

    pub fn is_open(&self, m: Menu) -> bool {
        !self.collapsed && self.open.last() == Some(&m)
    }

    pub fn toggle(&mut self, m: Menu) {
        if !self.collapsed && self.open.last() == Some(&m) {
            self.open.clear();
            self.collapsed = true;
        } else {
            self.open = vec![m];
            self.focus = Some(m);
            self.collapsed = false;
        }
    }

    pub fn reveal(&mut self, m: Menu) {
        self.open = vec![m];
        self.focus = Some(m);
        self.collapsed = false;
    }

    /// Selecciona exclusivamente esta sección en la columna adyacente izquierda.
    pub fn open(&mut self, m: Menu) {
        self.open = vec![m];
        self.collapsed = false;
    }

    #[allow(dead_code)]
    fn take_focus(&mut self, m: Menu) -> bool {
        if self.focus == Some(m) {
            self.focus = None;
            true
        } else {
            false
        }
    }

    /// Script semantics: exactly this section open (None = all collapsed),
    /// so `layout` dumps stay deterministic.
    pub fn set_exclusive(&mut self, m: Option<Menu>) {
        self.open = m.into_iter().collect();
        self.collapsed = self.open.is_empty();
    }

    pub fn set_open(&mut self, open: Vec<Menu>) {
        self.open = open;
        self.collapsed = self.open.is_empty();
    }

    pub fn open_list(&self) -> &[Menu] {
        &self.open
    }
}

/// Dibuja la columna secundaria adyacente a la derecha de la columna principal izquierda
/// (Estilo PicoScope 7 — Imagen 2), mostrando exclusivamente las opciones del ítem seleccionado.
#[allow(clippy::too_many_arguments)]
pub fn show(
    ctx: &egui::Context,
    l: &Layout,
    menus: &mut MenuState,
    link: &mut Link,
    phosphor: &mut Phosphor,
    math: &mut MathState,
    meas: &mut MeasureState,
    fft: &mut FftState,
    cur: &mut CursorState,
    pf: &mut PfState,
    rec: &mut Recorder,
    hist: &mut crate::record::History,
    refs: &mut crate::refs::RefState,
    script: &mut crate::script::Script,
    wf: &mut crate::viz::waterfall::WaterfallState,
    viz: &mut crate::viz::three_d::Viz3dState,
    fx: &crate::effects::Effects,
    ap: &mut crate::autopeak::AutoPeak,
    deep: &mut crate::deep::DeepView,
    dec: &mut crate::decode::DecodeState,
    auto: &mut crate::ui::automotive_dock::AutomotiveUiState,
) -> egui::Rect {
    if menus.is_collapsed() {
        return egui::Rect::NOTHING;
    }

    let Some(active_menu) = menus.open.last().copied() else {
        return egui::Rect::NOTHING;
    };

    let rect = l.points(Roi::Dialog.rect(l));
    let mut frame = egui::Frame::new();
    frame.fill = egui::Color32::from_rgb(14, 18, 26);
    frame.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(52, 64, 84));
    frame.inner_margin = 0.0.into();

    let mut close_flyout = false;
    let resp = egui::Area::new("dock".into())
        .fixed_pos(rect.min)
        .constrain(false)
        .show(ctx, |ui| {
            ui.set_clip_rect(rect);
            ui.set_max_width(rect.width());
            ui.set_min_width(rect.width());
            ui.set_max_height(rect.height());
            frame.show(ui, |ui| {
                ui.set_min_height(rect.height());

                // Barra de encabezado superior celeste estilo PicoScope 7 (Imagen 2)
                let hdr_h = 26.0;
                let (hdr_rect, _) =
                    ui.allocate_exact_size(egui::vec2(rect.width(), hdr_h), egui::Sense::hover());
                ui.painter().rect_filled(
                    hdr_rect,
                    0.0,
                    egui::Color32::from_rgb(86, 178, 245),
                );
                ui.painter().text(
                    hdr_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    active_menu.title(),
                    egui::FontId::proportional(12.5),
                    egui::Color32::from_rgb(10, 18, 28),
                );
                let close_rect = egui::Rect::from_min_size(
                    egui::pos2(hdr_rect.right() - 24.0, hdr_rect.top() + 2.0),
                    egui::vec2(22.0, 22.0),
                );
                if ui
                    .put(
                        close_rect,
                        egui::Button::new(
                            egui::RichText::new("✕")
                                .size(12.0)
                                .strong()
                                .color(egui::Color32::from_rgb(10, 18, 28)),
                        )
                        .frame(false),
                    )
                    .on_hover_text("Cerrar columna adyacente de configuración")
                    .clicked()
                {
                    close_flyout = true;
                }

                // Cuerpo desplazable con las opciones exclusivas del ítem seleccionado
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(8, 8))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(rect.height() - hdr_h - 16.0)
                            .max_width(rect.width() - 16.0)
                            .show(ui, |ui| {
                                ui.set_max_width(rect.width() - 20.0);
                                ui.set_min_width(rect.width() - 20.0);

                                match active_menu {
                                    Menu::Annotations => crate::ui::plot_tools::show_annotations_panel(ui, &mut auto.plot_tools, cur),
                                    Menu::More => crate::ui::suite::show_more_tools_panel(
                                        ui, link, math, menus, auto,
                                    ),
                                    Menu::Automotive => {
                                        crate::ui::automotive_dock::show(ui, auto, link, rec, hist)
                                    }
                                    Menu::Channel(ch) => {
                                        dialog_channel::show(ui, link, ch, auto)
                                    }
                                    Menu::Horizontal => {
                                        dialog_horizontal::show(ui, link, phosphor, deep)
                                    }
                                    Menu::Trigger => dialog_trigger::show(ui, link, auto),
                                    Menu::Acquire => dialog_acquire::show(ui, link, ap),
                                    Menu::Display => dialog_display::show(
                                        ui, link, phosphor, cur, refs, fft, wf, viz, fx, script,
                                    ),
                                    Menu::Measure => {
                                        meas.window = true;
                                        dialog_measure::show(ui, meas)
                                    }
                                    Menu::Math => dialog_math::show(ui, math),
                                    Menu::Cursor => {
                                        dialog_cursor::show(ui, link, cur, meas, deep, auto)
                                    }
                                    Menu::Decode => dialog_decode::show(ui, dec),
                                    Menu::Record => {
                                        dialog_record::show(ui, link, rec, hist, script)
                                    }
                                    Menu::Utility => {
                                        dialog_utility::show(ui, link, math, pf, fft, script)
                                    }
                                }
                            });
                    });
            });
        });
    if close_flyout {
        menus.set_collapsed(true);
    }
    l.pixels(resp.response.rect)
}

/// Always-visible zoom/pan/home strip at the top of the dock — the same
/// operations the plot gestures (`ui/touch.rs`) and the `zoom`/`pan`/`home`
/// script actions drive.
#[allow(dead_code)]
fn view_toolbar(ui: &mut egui::Ui, link: &mut crate::Link, phosphor: &mut crate::gpu::Phosphor) {
    use crate::ui::icons::{Icon, button};
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let sel = link.selected.min(1);
        if button(
            ui,
            Icon::ZoomOut,
            "Vertical zoom out — coarser V/div (selected channel)",
            26.0,
        )
        .clicked()
        {
            crate::view::zoom_channel(link, sel, false);
        }
        if button(
            ui,
            Icon::ZoomIn,
            "Vertical zoom in — finer V/div (selected channel)",
            26.0,
        )
        .clicked()
        {
            crate::view::zoom_channel(link, sel, true);
        }
        let zoomed = crate::view::zoom_active(phosphor);
        let (out_tip, in_tip) = if zoomed {
            (
                "Horizontal zoom out — wider zoom window",
                "Horizontal zoom in — narrower zoom window",
            )
        } else {
            (
                "Horizontal zoom out — slower time base (more s/div)",
                "Horizontal zoom in — faster time base (fewer s/div)",
            )
        };
        if button(ui, Icon::ZoomOut, out_tip, 26.0).clicked() {
            crate::view::hzoom(link, phosphor, phosphor.hview.0, false);
        }
        if button(ui, Icon::ZoomIn, in_tip, 26.0).clicked() {
            crate::view::hzoom(link, phosphor, phosphor.hview.0, true);
        }
        if button(
            ui,
            Icon::Home,
            "Home — default zoom and centre position (key: H)",
            26.0,
        )
        .clicked()
        {
            crate::view::home(link, phosphor);
        }
    });
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (icon, dir, tip) in [
            (
                Icon::ArrowLeft,
                crate::view::Pan::Left,
                "Pan left (record window)",
            ),
            (
                Icon::ArrowRight,
                crate::view::Pan::Right,
                "Pan right (record window)",
            ),
            (
                Icon::ArrowUp,
                crate::view::Pan::Up,
                "Pan up (selected channel offset)",
            ),
            (
                Icon::ArrowDown,
                crate::view::Pan::Down,
                "Pan down (selected channel offset)",
            ),
        ] {
            if button(ui, icon, tip, 26.0).clicked() {
                crate::view::pan(link, phosphor, dir);
            }
        }
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("VIEW — drag plot to pan, scroll to zoom")
                .size(9.0)
                .color(egui::Color32::GRAY),
        );
    });
    ui.add_space(4.0);
}

fn section(
    ui: &mut egui::Ui,
    menus: &mut MenuState,
    m: Menu,
    body: impl FnOnce(&mut egui::Ui, &mut MenuState),
) {
    let open = menus.is_open(m);
    // The caret is painted, not a glyph: egui's bundled fonts are subset per
    // platform and the triangles rendered as tofu on some of them, the same
    // failure the toolbar icons were replaced for.
    let header = egui::Button::new(
        egui::RichText::new(format!("    {}", m.title()))
            .strong()
            .size(12.0),
    )
    .fill(if open {
        egui::Color32::from_rgb(30, 34, 44)
    } else {
        egui::Color32::from_rgb(20, 22, 28)
    })
    .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(55)))
    .min_size(egui::vec2(ui.available_width(), 24.0));
    let resp = ui.add(header);
    caret(ui, resp.rect, open);
    if resp.clicked() {
        menus.toggle(m);
    }
    // Opened from somewhere else (a front-panel key, the View menu): bring
    // it into view, or the click looks like it did nothing.
    if menus.take_focus(m) {
        ui.scroll_to_rect(resp.rect, Some(egui::Align::TOP));
    }
    if menus.is_open(m) {
        ui.add_space(4.0);
        body(ui, menus);
        ui.add_space(6.0);
    }
}

fn caret(ui: &egui::Ui, rect: egui::Rect, open: bool) {
    let c = egui::pos2(rect.left() + 12.0, rect.center().y);
    let r = 4.0;
    let pts = if open {
        vec![
            egui::pos2(c.x - r, c.y - r * 0.6),
            egui::pos2(c.x + r, c.y - r * 0.6),
            egui::pos2(c.x, c.y + r * 0.8),
        ]
    } else {
        vec![
            egui::pos2(c.x - r * 0.6, c.y - r),
            egui::pos2(c.x - r * 0.6, c.y + r),
            egui::pos2(c.x + r * 0.8, c.y),
        ]
    };
    ui.painter().add(egui::Shape::convex_polygon(
        pts,
        egui::Color32::from_gray(190),
        egui::Stroke::NONE,
    ));
}
