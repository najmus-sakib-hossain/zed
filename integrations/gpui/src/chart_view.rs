//! 📊 Chart Renderer Component
//!
//! Strategy: `plotters` renders charts to an in-memory bitmap buffer → RGBA → GPUI.
//! Supports Line, Bar, Scatter, and Area chart types.

use gpui::*;
use gpui_component::StyledExt;
use image::RgbaImage;
use plotters::prelude::*;
use std::sync::{Arc, Mutex};

use crate::bitmap_utils;

pub struct ChartView {
    rendered: Arc<Mutex<Option<RgbaImage>>>,
    chart_type: ChartType,
    data: Vec<(f64, f64)>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ChartType {
    Line,
    Bar,
    Scatter,
    Area,
}

impl ChartView {
    pub fn new(_window: &Window, _cx: &mut Context<Self>) -> Self {
        // Sample data
        let data: Vec<(f64, f64)> = (0..50)
            .map(|i| {
                let x = i as f64 * 0.2;
                let y = (x * 0.5).sin() * 50.0 + 50.0 + (x * 1.5).cos() * 20.0;
                (x, y)
            })
            .collect();

        let mut view = Self {
            rendered: Arc::new(Mutex::new(None)),
            chart_type: ChartType::Line,
            data,
        };

        view.render_chart();
        view
    }

    fn render_chart(&mut self) {
        let width = 800u32;
        let height = 500u32;
        let data = self.data.clone();
        let chart_type = self.chart_type;

        // Render chart to in-memory RGBA buffer
        let img = render_chart_to_rgba(width, height, &data, chart_type);
        *self.rendered.lock().unwrap() = img.ok();
    }

    fn set_chart_type(&mut self, ct: ChartType, cx: &mut Context<Self>) {
        self.chart_type = ct;
        self.render_chart();
        cx.notify();
    }
}

impl Render for ChartView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rendered = self.rendered.lock().unwrap();

        div()
            .v_flex()
            .gap_3()
            .items_center()
            .p_4()
            .child(
                div()
                    .text_color(rgb(0xcdd6f4))
                    .text_xl()
                    .child("📊 Chart Renderer (plotters — pure Rust)"),
            )
            // Chart type selector
            .child(
                div()
                    .h_flex()
                    .gap_2()
                    .children(
                        [
                            ("📈 Line",    ChartType::Line),
                            ("📊 Bar",     ChartType::Bar),
                            ("⚬ Scatter",  ChartType::Scatter),
                            ("▓ Area",     ChartType::Area),
                        ]
                        .into_iter()
                        .map(|(label, ct)| {
                            let is_active = self.chart_type == ct;
                            let switch = cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                                this.set_chart_type(ct, cx);
                            });
                            div()
                                .px_3()
                                .py_1()
                                .bg(if is_active { rgb(0x89b4fa) } else { rgb(0x313244) })
                                .text_color(if is_active {
                                    rgb(0x1e1e2e)
                                } else {
                                    rgb(0xcdd6f4)
                                })
                                .rounded(px(6.))
                                .cursor_pointer()
                                .child(label)
                                .on_mouse_down(MouseButton::Left, switch)
                        }),
                    ),
            )
            // Chart display
            .child(
                div()
                    .w(px(800.))
                    .h(px(500.))
                    .bg(rgb(0xffffff))
                    .rounded(px(8.))
                    .shadow_lg()
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(if let Some(ref img_data) = *rendered {
                        let source = bitmap_utils::rgba_to_gpui_image(img_data);
                        div().child(
                            img(source)
                                .w(px(800.))
                                .h(px(500.))
                                .object_fit(ObjectFit::Contain),
                        )
                    } else {
                        div()
                            .text_color(rgb(0x6c7086))
                            .child("Rendering chart...")
                    }),
            )
    }
}

/// Render a chart to an in-memory RGBA image using plotters-bitmap
fn render_chart_to_rgba(
    width: u32,
    height: u32,
    data: &[(f64, f64)],
    chart_type: ChartType,
) -> anyhow::Result<RgbaImage> {
    // Create an in-memory RGB buffer
    let mut pixel_buf = vec![0u8; (width * height * 3) as usize];

    {
        let backend =
            BitMapBackend::with_buffer(&mut pixel_buf, (width, height));
        let root = backend.into_drawing_area();
        root.fill(&WHITE)?;

        // Compute data range
        let x_min = data.iter().map(|d| d.0).fold(f64::INFINITY, f64::min);
        let x_max = data.iter().map(|d| d.0).fold(f64::NEG_INFINITY, f64::max);
        let y_min = data.iter().map(|d| d.1).fold(f64::INFINITY, f64::min);
        let y_max = data.iter().map(|d| d.1).fold(f64::NEG_INFINITY, f64::max);
        let y_pad = (y_max - y_min) * 0.1;

        let mut chart = ChartBuilder::on(&root)
            .caption(
                match chart_type {
                    ChartType::Line => "Line Chart",
                    ChartType::Bar => "Bar Chart",
                    ChartType::Scatter => "Scatter Plot",
                    ChartType::Area => "Area Chart",
                },
                ("sans-serif", 24).into_font(),
            )
            .margin(20)
            .x_label_area_size(40)
            .y_label_area_size(50)
            .build_cartesian_2d(x_min..x_max, (y_min - y_pad)..(y_max + y_pad))?;

        chart
            .configure_mesh()
            .x_desc("X Axis")
            .y_desc("Y Axis")
            .draw()?;

        match chart_type {
            ChartType::Line => {
                chart.draw_series(LineSeries::new(
                    data.iter().copied(),
                    &BLUE,
                ))?
                .label("Data")
                .legend(|(x, y)| {
                    PathElement::new(vec![(x, y), (x + 20, y)], BLUE)
                });
            }
            ChartType::Bar => {
                let bar_width = if data.len() > 1 {
                    (data[1].0 - data[0].0) * 0.8
                } else {
                    0.5
                };
                chart.draw_series(data.iter().map(|&(x, y)| {
                    let x0 = x - bar_width / 2.0;
                    let x1 = x + bar_width / 2.0;
                    Rectangle::new(
                        [(x0, 0.0), (x1, y)],
                        RGBColor(66, 133, 244).filled(),
                    )
                }))?;
            }
            ChartType::Scatter => {
                chart.draw_series(data.iter().map(|&(x, y)| {
                    Circle::new((x, y), 4, RED.filled())
                }))?;
            }
            ChartType::Area => {
                chart.draw_series(AreaSeries::new(
                    data.iter().copied(),
                    0.0,
                    RGBColor(66, 133, 244).mix(0.3),
                ))?;
                // Overlay line
                chart.draw_series(LineSeries::new(
                    data.iter().copied(),
                    &BLUE,
                ))?;
            }
        }

        chart
            .configure_series_labels()
            .background_style(WHITE.mix(0.8))
            .border_style(BLACK)
            .draw()?;

        root.present()?;
    }

    // Convert RGB → RGBA
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for chunk in pixel_buf.chunks(3) {
        rgba.push(chunk[0]);
        rgba.push(chunk[1]);
        rgba.push(chunk[2]);
        rgba.push(255);
    }

    RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| anyhow::anyhow!("Failed to create chart image"))
}
