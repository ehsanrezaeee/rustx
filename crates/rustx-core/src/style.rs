#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Div,
    Text,
    Button,
    Image,
    Input,
    Canvas,
    Fragment,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Column,
    Row,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Display {
    #[default]
    Flex,
    Grid,
    Block,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub color: u32,
    pub blur: f32,
    pub offset: (f32, f32),
}
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub direction: Direction,
    pub gap: f32,
    pub padding: f32,
    pub background: Option<u32>,
    pub color: u32,
    pub font_size: f32,
    pub small_font_size: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub display: Display,
    pub columns: u16,
    pub min_column: f32,
    pub grow: f32,
    pub wrap: bool,
    pub clip: bool,
    pub radius: f32,
    pub border: Option<(f32, u32)>,
    pub gradient: Option<(u32, u32)>,
    pub shadow: Option<Shadow>,
    pub opacity: f32,
    pub translate: (f32, f32),
}
impl Default for Style {
    fn default() -> Self {
        Self {
            direction: Direction::Column,
            gap: 8.,
            padding: 0.,
            background: None,
            color: 0xe6eaf2,
            font_size: 16.,
            small_font_size: None,
            width: None,
            height: None,
            display: Display::Flex,
            columns: 1,
            min_column: 0.,
            grow: 0.,
            wrap: false,
            clip: false,
            radius: 0.,
            border: None,
            gradient: None,
            shadow: None,
            opacity: 1.,
            translate: (0., 0.),
        }
    }
}
impl Style {
    pub fn small_font_size(mut self, v: f32) -> Self {
        self.small_font_size = Some(v);
        self
    }

    pub fn height(mut self, v: f32) -> Self {
        self.height = Some(v.max(1.));
        self
    }
    pub fn grid(columns: u16) -> Self {
        Self {
            display: Display::Grid,
            columns: columns.max(1),
            ..Self::default()
        }
    }
    pub fn block() -> Self {
        Self {
            display: Display::Block,
            ..Self::default()
        }
    }
    pub fn responsive(mut self, min_column: f32) -> Self {
        self.min_column = min_column.max(1.);
        self
    }
    pub fn grow(mut self, v: f32) -> Self {
        self.grow = v.max(0.);
        self
    }
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }
    pub fn clip(mut self) -> Self {
        self.clip = true;
        self
    }
    pub fn radius(mut self, v: f32) -> Self {
        self.radius = v.max(0.);
        self
    }
    pub fn border(mut self, width: f32, color: u32) -> Self {
        self.border = Some((width.max(0.), color));
        self
    }
    pub fn gradient(mut self, start: u32, end: u32) -> Self {
        self.gradient = Some((start, end));
        self
    }
    pub fn shadow(mut self, color: u32, blur: f32, offset: (f32, f32)) -> Self {
        self.shadow = Some(Shadow {
            color,
            blur: blur.max(0.),
            offset,
        });
        self
    }
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0., 1.);
        self
    }
    pub fn translate(mut self, x: f32, y: f32) -> Self {
        self.translate = (x, y);
        self
    }
    pub fn column() -> Self {
        Self::default()
    }
    pub fn row() -> Self {
        Self {
            direction: Direction::Row,
            ..Self::default()
        }
    }
    pub fn gap(mut self, v: f32) -> Self {
        self.gap = v.max(0.);
        self
    }
    pub fn padding(mut self, v: f32) -> Self {
        self.padding = v.max(0.);
        self
    }
    pub fn background(mut self, v: u32) -> Self {
        self.background = Some(v);
        self
    }
    pub fn color(mut self, v: u32) -> Self {
        self.color = v;
        self
    }
    pub fn font_size(mut self, v: f32) -> Self {
        self.font_size = v.clamp(8., 64.);
        self
    }
    pub fn width(mut self, v: f32) -> Self {
        self.width = Some(v.max(1.));
        self
    }
}
