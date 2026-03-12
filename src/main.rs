use iced::alignment;
use iced::advanced::image::Renderer as ImageRenderer;
use iced::mouse;
use iced::widget::canvas::{
    self, path::arc, Action, Fill, Frame, Geometry, Path, Program, Stroke,
};
use iced::widget::{
    button, canvas as canvas_widget, column, container, row, slider, text,
};
use iced::{
    Alignment, Border, Color, ContentFit, Element, Length, Point, Radians, Rectangle, Renderer,
    Task, Theme, Vector,
};
use iced_aw::{
    menu::{DrawPath, Menu as IcedMenu},
    menu_bar, menu_items,
};
use rfd::AsyncFileDialog;
use std::path::{Path as FsPath, PathBuf};

fn main() -> iced::Result {
    iced::application(Editor::default, update, view)
        .title(title)
        .theme(theme)
        .run()
}

fn title(_editor: &Editor) -> String {
    "iomha".to_string()
}

fn theme(_editor: &Editor) -> Theme {
    Theme::Dark
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tool {
    Select,
    Brush,
    Rectangle,
    Ellipse,
}

impl Tool {
    fn label(self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Brush => "Brush",
            Tool::Rectangle => "Rectangle",
            Tool::Ellipse => "Ellipse",
        }
    }
}

#[derive(Debug, Clone)]
struct Layer {
    visible: bool,
}

#[derive(Debug, Clone)]
enum ShapeKind {
    BrushStroke(Vec<Point>),
    Rectangle { start: Point, end: Point },
    Ellipse { start: Point, end: Point },
}

#[derive(Debug, Clone)]
struct Shape {
    layer: usize,
    kind: ShapeKind,
    stroke: Color,
    fill: Color,
    stroke_width: f32,
}

#[derive(Debug, Clone)]
struct DraftShape {
    tool: Tool,
    origin: Point,
    current: Point,
    points: Vec<Point>,
}

#[derive(Debug, Clone)]
struct LoadedImage {
    path: PathBuf,
    handle: iced::widget::image::Handle,
}

#[derive(Debug, Clone)]
enum Message {
    None,
    OpenFileRequested,
    OpenFileSelected(Option<PathBuf>),
    ToolSelected(Tool),
    StrokeWidthChanged(f32),
    CanvasPressed(Point),
    CanvasDragged(Point),
    CanvasReleased(Point),
}

struct Editor {
    tool: Tool,
    stroke_width: f32,
    layers: Vec<Layer>,
    active_layer: usize,
    shapes: Vec<Shape>,
    draft: Option<DraftShape>,
    loaded_image: Option<LoadedImage>,
    status: String,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            tool: Tool::Brush,
            stroke_width: 6.0,
            layers: vec![
                Layer { visible: true },
                Layer { visible: true },
            ],
            active_layer: 1,
            shapes: vec![],
            draft: None,
            loaded_image: None,
            status: "File -> Open loads PNG, JPEG, JPG, and BMP images.".into(),
        }
    }
}

fn update(editor: &mut Editor, message: Message) -> Task<Message> {
    match message {
        Message::None => {}
        Message::OpenFileRequested => {
            editor.status = "Choose an image to open.".into();
            return Task::perform(
                async {
                    AsyncFileDialog::new()
                        .set_title("Open image")
                        .add_filter("Images", &["png", "jpeg", "jpg", "bmp"])
                        .pick_file()
                        .await
                        .map(|file| file.path().to_path_buf())
                },
                Message::OpenFileSelected,
            );
        }
        Message::OpenFileSelected(Some(path)) => {
            let handle = iced::widget::image::Handle::from_path(&path);
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("image")
                .to_string();
            editor.loaded_image = Some(LoadedImage { path, handle });
            editor.status = format!("Opened {}.", file_name);
        }
        Message::OpenFileSelected(None) => {
            editor.status = "Open cancelled.".into();
        }
        Message::ToolSelected(tool) => {
            editor.tool = tool;
            editor.status = match tool {
                Tool::Select => "Select mode is still a placeholder.".into(),
                Tool::Brush => "Brush ready. Drag on the artboard to paint.".into(),
                Tool::Rectangle => "Rectangle tool ready. Drag to place a shape.".into(),
                Tool::Ellipse => "Ellipse tool ready. Drag to place a shape.".into(),
            };
        }
        Message::StrokeWidthChanged(width) => {
            editor.stroke_width = width;
            editor.status = format!("Stroke width set to {:.0}px.", width);
        }
        Message::CanvasPressed(point) => {
            if editor.tool == Tool::Select {
                editor.status = "Select mode is not implemented yet.".into();
                return Task::none();
            }

            let mut draft = DraftShape {
                tool: editor.tool,
                origin: point,
                current: point,
                points: vec![point],
            };

            if editor.tool != Tool::Brush {
                draft.points.clear();
            }

            editor.draft = Some(draft);
        }
        Message::CanvasDragged(point) => {
            if let Some(draft) = editor.draft.as_mut() {
                draft.current = point;

                if draft.tool == Tool::Brush {
                    let should_push = draft
                        .points
                        .last()
                        .map(|last| distance(*last, point) > 1.5)
                        .unwrap_or(true);

                    if should_push {
                        draft.points.push(point);
                    }
                }
            }
        }
        Message::CanvasReleased(point) => {
            if let Some(mut draft) = editor.draft.take() {
                draft.current = point;

                let shape = match draft.tool {
                    Tool::Brush => {
                        if draft.points.len() < 2 {
                            editor.status = "Ignored a brush stroke that was too short.".into();
                            return Task::none();
                        }

                        Shape {
                            layer: editor.active_layer,
                            kind: ShapeKind::BrushStroke(draft.points),
                            stroke: palette_color(0),
                            fill: Color::TRANSPARENT,
                            stroke_width: editor.stroke_width,
                        }
                    }
                    Tool::Rectangle => Shape {
                        layer: editor.active_layer,
                        kind: ShapeKind::Rectangle {
                            start: draft.origin,
                            end: point,
                        },
                        stroke: palette_color(0),
                        fill: palette_color(6),
                        stroke_width: editor.stroke_width,
                    },
                    Tool::Ellipse => Shape {
                        layer: editor.active_layer,
                        kind: ShapeKind::Ellipse {
                            start: draft.origin,
                            end: point,
                        },
                        stroke: palette_color(1),
                        fill: palette_color(7),
                        stroke_width: editor.stroke_width,
                    },
                    Tool::Select => {
                        return Task::none();
                    }
                };

                editor.shapes.push(shape);
                editor.status = format!("{} committed.", draft.tool.label());
            }
        }
    }

    Task::none()
}

fn view(editor: &Editor) -> Element<'_, Message> {
    let menu = menu_bar(editor);

    let toolbar = container(
        row![
            tool_button(editor, Tool::Select),
            tool_button(editor, Tool::Brush),
            tool_button(editor, Tool::Rectangle),
            tool_button(editor, Tool::Ellipse),
            text("Stroke").size(14),
            slider(1.0..=32.0, editor.stroke_width, Message::StrokeWidthChanged).width(180),
            text(format!("{:.0}px", editor.stroke_width)).size(14),
            text(match &editor.loaded_image {
                Some(image) => short_path(&image.path),
                None => "No image loaded".to_string(),
            })
            .size(14)
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding([10, 16])
    .style(panel_style);

    let canvas = container(
        canvas_widget(EditorCanvas { editor })
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .padding(16)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(workspace_style);

    let status = container(text(&editor.status))
        .padding([10, 20])
        .style(panel_style);

    column![menu, toolbar, canvas, status]
        .height(Length::Fill)
        .into()
}

fn menu_bar(editor: &Editor) -> Element<'_, Message> {
    let image_label = match &editor.loaded_image {
        Some(image) => format!("Loaded: {}", short_path(&image.path)),
        None => "Loaded: none".to_string(),
    };

    let menu_tpl = |items| IcedMenu::new(items).width(220.0).offset(12.0).spacing(4.0);

    #[rustfmt::skip]
    let menu = menu_bar!(
        (menu_dropdown("File", Message::None), {
            menu_tpl(menu_items!(
                (menu_item("Open", Message::OpenFileRequested)),
            ))
        }),
        (menu_dropdown("Image", Message::None), {
            menu_tpl(menu_items!(
                (menu_item(image_label, Message::None)),
                (menu_item("Supported: png, jpeg, jpg, bmp", Message::None)),
            ))
        }),
    )
    .draw_path(DrawPath::Backdrop)
    .close_on_item_click_global(true)
    .width(Length::Fill);

    menu.into()
}

fn base_button<'a>(
    content: impl Into<Element<'a, Message>>,
    msg: Message,
) -> button::Button<'a, Message> {
    button(content)
        .padding([4, 8])
        .style(|theme, status| {
            use button::{Status, Style};

            let palette = theme.extended_palette();
            let base = Style {
                text_color: palette.background.base.text,
                border: Border::default().rounded(6.0),
                ..Style::default()
            };

            match status {
                Status::Active => base.with_background(Color::TRANSPARENT),
                Status::Hovered => base.with_background(Color::from_rgb(
                    palette.primary.weak.color.r * 1.2,
                    palette.primary.weak.color.g * 1.2,
                    palette.primary.weak.color.b * 1.2,
                )),
                Status::Disabled => base.with_background(Color::from_rgb(0.35, 0.35, 0.35)),
                Status::Pressed => base.with_background(palette.primary.weak.color),
            }
        })
        .on_press(msg)
}

fn menu_button(
    label: impl Into<String>,
    width: Option<Length>,
    height: Option<Length>,
    msg: Message,
) -> Element<'static, Message> {
    base_button(
        text(label.into())
            .height(height.unwrap_or(Length::Shrink))
            .align_y(alignment::Vertical::Center),
        msg,
    )
    .width(width.unwrap_or(Length::Shrink))
    .height(height.unwrap_or(Length::Shrink))
    .into()
}

fn menu_dropdown(label: impl Into<String>, message: Message) -> Element<'static, Message> {
    menu_button(label, Some(Length::Shrink), Some(Length::Shrink), message)
}

fn menu_item(label: impl Into<String>, message: Message) -> Element<'static, Message> {
    menu_button(label, Some(Length::Fill), Some(Length::Shrink), message)
}

fn tool_button(editor: &Editor, tool: Tool) -> Element<'_, Message> {
    let label = if editor.tool == tool {
        format!("[{}]", tool.label())
    } else {
        tool.label().to_string()
    };

    base_button(text(label), Message::ToolSelected(tool)).into()
}

fn panel_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgb8(22, 27, 34))),
        border: Border {
            width: 1.0,
            color: Color::from_rgba8(148, 163, 184, 0.16),
            ..Default::default()
        },
        text_color: Some(theme.palette().text),
        ..Default::default()
    }
}

fn workspace_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgb8(13, 17, 23))),
        ..Default::default()
    }
}

struct EditorCanvas<'a> {
    editor: &'a Editor,
}

impl Program<Message> for EditorCanvas<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, frame.size(), Color::from_rgb8(12, 16, 22));

        let artboard = artboard_bounds(bounds);
        let artboard_path = Path::rectangle(artboard.position(), artboard.size());
        frame.fill(&artboard_path, Color::from_rgb8(236, 240, 246));
        frame.stroke(
            &artboard_path,
            Stroke::default()
                .with_color(Color::from_rgba8(207, 216, 230, 0.18))
                .with_width(1.0),
        );

        draw_grid(&mut frame, artboard);

        if let Some(image) = &self.editor.loaded_image {
            frame.draw_image(
                fit_image_into_artboard(renderer, &image.handle, artboard),
                &image.handle,
            );
        }

        for shape in self.editor.shapes.iter().filter(|shape| {
            self.editor
                .layers
                .get(shape.layer)
                .map(|layer| layer.visible)
                .unwrap_or(false)
        }) {
            draw_shape(&mut frame, shape);
        }

        if let Some(draft) = &self.editor.draft {
            let preview = match draft.tool {
                Tool::Brush => Shape {
                    layer: self.editor.active_layer,
                    kind: ShapeKind::BrushStroke(draft.points.clone()),
                    stroke: palette_color(0),
                    fill: Color::TRANSPARENT,
                    stroke_width: self.editor.stroke_width,
                },
                Tool::Rectangle => Shape {
                    layer: self.editor.active_layer,
                    kind: ShapeKind::Rectangle {
                        start: draft.origin,
                        end: draft.current,
                    },
                    stroke: palette_color(0),
                    fill: palette_color(6).scale_alpha(0.35),
                    stroke_width: self.editor.stroke_width,
                },
                Tool::Ellipse => Shape {
                    layer: self.editor.active_layer,
                    kind: ShapeKind::Ellipse {
                        start: draft.origin,
                        end: draft.current,
                    },
                    stroke: palette_color(1),
                    fill: palette_color(7).scale_alpha(0.35),
                    stroke_width: self.editor.stroke_width,
                },
                Tool::Select => return vec![frame.into_geometry()],
            };

            draw_shape(&mut frame, &preview);
        }

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        _state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        let Some(position) = cursor.position_in(bounds) else {
            return None;
        };

        if !rectangle_contains(artboard_bounds(bounds), position) {
            return None;
        }

        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                Some(Action::publish(Message::CanvasPressed(position)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) if self.editor.draft.is_some() => {
                Some(Action::publish(Message::CanvasDragged(position)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if self.editor.draft.is_some() =>
            {
                Some(Action::publish(Message::CanvasReleased(position)).and_capture())
            }
            _ => None,
        }
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor
            .position_in(bounds)
            .is_some_and(|position| rectangle_contains(artboard_bounds(bounds), position))
        {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

fn draw_grid(frame: &mut Frame<Renderer>, artboard: Rectangle) {
    let columns = 12;
    let rows = 10;

    for i in 1..columns {
        let x = artboard.x + artboard.width * (i as f32 / columns as f32);
        frame.stroke(
            &Path::line(Point::new(x, artboard.y), Point::new(x, artboard.y + artboard.height)),
            Stroke::default().with_color(Color::from_rgba8(15, 23, 42, 0.08)),
        );
    }

    for i in 1..rows {
        let y = artboard.y + artboard.height * (i as f32 / rows as f32);
        frame.stroke(
            &Path::line(Point::new(artboard.x, y), Point::new(artboard.x + artboard.width, y)),
            Stroke::default().with_color(Color::from_rgba8(15, 23, 42, 0.08)),
        );
    }
}

fn fit_image_into_artboard(
    renderer: &Renderer,
    handle: &iced::widget::image::Handle,
    artboard: Rectangle,
) -> Rectangle {
    let size = renderer.measure_image(handle).unwrap_or_default();
    let size = iced::Size::new(size.width as f32, size.height as f32);

    if size.width <= 0.0 || size.height <= 0.0 {
        return artboard;
    }

    let fitted = ContentFit::Contain.fit(size, artboard.size());
    Rectangle {
        x: artboard.x + (artboard.width - fitted.width) / 2.0,
        y: artboard.y + (artboard.height - fitted.height) / 2.0,
        width: fitted.width,
        height: fitted.height,
    }
}

fn artboard_bounds(bounds: Rectangle) -> Rectangle {
    let padding_x = 40.0;
    let padding_y = 32.0;

    Rectangle {
        x: padding_x,
        y: padding_y,
        width: (bounds.width - padding_x * 2.0).max(1.0),
        height: (bounds.height - padding_y * 2.0).max(1.0),
    }
}

fn draw_shape(frame: &mut Frame<Renderer>, shape: &Shape) {
    match &shape.kind {
        ShapeKind::BrushStroke(points) => {
            if points.len() < 2 {
                return;
            }

            let path = Path::new(|builder| {
                builder.move_to(points[0]);
                for point in points.iter().skip(1) {
                    builder.line_to(*point);
                }
            });

            frame.stroke(
                &path,
                Stroke::default()
                    .with_color(shape.stroke)
                    .with_width(shape.stroke_width)
                    .with_line_cap(canvas::LineCap::Round)
                    .with_line_join(canvas::LineJoin::Round),
            );
        }
        ShapeKind::Rectangle { start, end } => {
            let bounds = rectangle_from_points(*start, *end);
            let rect = Path::rectangle(bounds.position(), bounds.size());
            frame.fill(&rect, Fill::from(shape.fill));
            frame.stroke(
                &rect,
                Stroke::default()
                    .with_color(shape.stroke)
                    .with_width(shape.stroke_width),
            );
        }
        ShapeKind::Ellipse { start, end } => {
            let bounds = rectangle_from_points(*start, *end);
            let ellipse = Path::new(|builder| {
                builder.ellipse(arc::Elliptical {
                    center: Point::new(
                        bounds.x + bounds.width / 2.0,
                        bounds.y + bounds.height / 2.0,
                    ),
                    radii: Vector::new(bounds.width / 2.0, bounds.height / 2.0),
                    rotation: Radians(0.0),
                    start_angle: Radians(0.0),
                    end_angle: Radians(std::f32::consts::TAU),
                });
                builder.close();
            });
            frame.fill(&ellipse, Fill::from(shape.fill));
            frame.stroke(
                &ellipse,
                Stroke::default()
                    .with_color(shape.stroke)
                    .with_width(shape.stroke_width),
            );
        }
    }
}

fn rectangle_from_points(start: Point, end: Point) -> Rectangle {
    let x = start.x.min(end.x);
    let y = start.y.min(end.y);
    let width = (start.x - end.x).abs().max(1.0);
    let height = (start.y - end.y).abs().max(1.0);

    Rectangle {
        x,
        y,
        width,
        height,
    }
}

fn rectangle_contains(rect: Rectangle, point: Point) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}

fn distance(a: Point, b: Point) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

fn short_path(path: &FsPath) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("untitled")
        .to_string()
}

const PALETTE: [[u8; 3]; 8] = [
    [28, 37, 51],
    [230, 87, 78],
    [240, 177, 76],
    [124, 170, 83],
    [72, 136, 207],
    [110, 116, 130],
    [238, 196, 197],
    [170, 219, 211],
];

fn palette_color(index: usize) -> Color {
    let [r, g, b] = PALETTE[index];
    Color::from_rgb8(r, g, b)
}
