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

/// Percentages use the familiar 0–100 scale and resolve against the parent content box.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Length {
    #[default]
    Auto,
    Px(f32),
    Percent(f32),
}

impl Length {
    pub fn resolve(self, parent: f32) -> Option<f32> {
        match self {
            Self::Auto => None,
            Self::Px(value) => Some(value),
            Self::Percent(value) => Some(parent * value / 100.),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Insets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Insets {
    pub const fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub const fn all(value: f32) -> Self {
        Self::new(value, value, value, value)
    }

    fn checked(self, allow_negative: bool) -> Self {
        let check = |value: f32| {
            assert!(value.is_finite(), "Spacing must be finite");
            if allow_negative {
                value
            } else {
                value.max(0.)
            }
        };
        Self::new(
            check(self.top),
            check(self.right),
            check(self.bottom),
            check(self.left),
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Position {
    #[default]
    Relative,
    Absolute,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Typography {
    pub family: Option<String>,
    pub weight: u16,
    pub line_height: Option<f32>,
    /// A visual limit for ordinary text. Editing always retains the complete shaped model.
    pub max_lines: Option<u16>,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            family: None,
            weight: 400,
            line_height: None,
            max_lines: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub width_length: Length,
    pub height_length: Length,
    pub min_height: Option<f32>,
    pub max_height: Option<f32>,
    pub padding_edges: Option<Insets>,
    pub logical_padding: Option<Insets>,
    pub margin: Insets,
    pub logical_margin: Option<Insets>,
    pub position: Position,
    /// Physical top/right/bottom/left offsets, independent of text direction.
    pub inset: [Length; 4],
    pub z_index: i32,
    pub typography: Typography,
    pub align: Align,
    pub justify: Justify,
    pub shrink: f32,
    pub min_width: Option<f32>,
    pub max_width: Option<f32>,
    pub text_align: TextAlign,
    pub text_direction: TextDirection,
    pub background_token: Option<ColorToken>,
    pub color_token: Option<ColorToken>,
    pub queries: Vec<ResponsiveStyle>,
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
            width_length: Length::Auto,
            height_length: Length::Auto,
            min_height: None,
            max_height: None,
            padding_edges: None,
            logical_padding: None,
            margin: Insets::default(),
            logical_margin: None,
            position: Position::Relative,
            inset: [Length::Auto; 4],
            z_index: 0,
            typography: Typography::default(),
            align: Align::Stretch,
            justify: Justify::Start,
            shrink: 0.,
            min_width: None,
            max_width: None,
            text_align: TextAlign::Start,
            text_direction: TextDirection::Auto,
            background_token: None,
            color_token: None,
            queries: vec![],
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
    pub fn width_length(mut self, value: Length) -> Self {
        self.width = None;
        self.width_length = value;
        self
    }

    pub fn height_length(mut self, value: Length) -> Self {
        self.height = None;
        self.height_length = value;
        self
    }

    pub fn width_percent(self, value: f32) -> Self {
        assert!(value.is_finite() && value >= 0.);
        self.width_length(Length::Percent(value))
    }

    pub fn height_percent(self, value: f32) -> Self {
        assert!(value.is_finite() && value >= 0.);
        self.height_length(Length::Percent(value))
    }

    pub fn min_height(mut self, value: f32) -> Self {
        self.min_height = Some(value.max(0.));
        self
    }

    pub fn max_height(mut self, value: f32) -> Self {
        self.max_height = Some(value.max(0.));
        self
    }

    pub fn padding_edges(mut self, value: Insets) -> Self {
        self.padding_edges = Some(value.checked(false));
        self.logical_padding = None;
        self
    }

    /// Block start, inline end, block end, inline start; vertical writing is outside v1.
    pub fn padding_logical(
        mut self,
        block_start: f32,
        inline_end: f32,
        block_end: f32,
        inline_start: f32,
    ) -> Self {
        self.logical_padding =
            Some(Insets::new(block_start, inline_end, block_end, inline_start).checked(false));
        self
    }

    pub fn margin(mut self, value: Insets) -> Self {
        self.margin = value.checked(true);
        self.logical_margin = None;
        self
    }

    pub fn margin_logical(
        mut self,
        block_start: f32,
        inline_end: f32,
        block_end: f32,
        inline_start: f32,
    ) -> Self {
        self.logical_margin =
            Some(Insets::new(block_start, inline_end, block_end, inline_start).checked(true));
        self
    }

    pub fn padding_insets(&self) -> Insets {
        let mut value = self
            .logical_padding
            .or(self.padding_edges)
            .unwrap_or(Insets::all(self.padding));
        if self.logical_padding.is_some() && self.text_direction == TextDirection::RightToLeft {
            std::mem::swap(&mut value.left, &mut value.right);
        }
        value
    }

    pub fn margin_insets(&self) -> Insets {
        let mut value = self.logical_margin.unwrap_or(self.margin);
        if self.logical_margin.is_some() && self.text_direction == TextDirection::RightToLeft {
            std::mem::swap(&mut value.left, &mut value.right);
        }
        value
    }

    pub fn padding_left(&self) -> f32 {
        self.padding_insets().left
    }
    pub fn padding_top(&self) -> f32 {
        self.padding_insets().top
    }
    pub fn padding_horizontal(&self) -> f32 {
        let edges = self.padding_insets();
        edges.left + edges.right
    }
    pub fn padding_vertical(&self) -> f32 {
        let edges = self.padding_insets();
        edges.top + edges.bottom
    }

    pub fn absolute(mut self) -> Self {
        self.position = Position::Absolute;
        self
    }
    pub fn top(mut self, value: Length) -> Self {
        self.inset[0] = value;
        self
    }
    pub fn right(mut self, value: Length) -> Self {
        self.inset[1] = value;
        self
    }
    pub fn bottom(mut self, value: Length) -> Self {
        self.inset[2] = value;
        self
    }
    pub fn left(mut self, value: Length) -> Self {
        self.inset[3] = value;
        self
    }
    pub fn z_index(mut self, value: i32) -> Self {
        self.z_index = value;
        self
    }

    pub fn font_family(mut self, value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(
            !value.is_empty() && value.len() <= 128,
            "Font family must be 1–128 bytes"
        );
        self.typography.family = Some(value);
        self
    }

    pub fn font_weight(mut self, value: u16) -> Self {
        self.typography.weight = value.clamp(1, 1000);
        self
    }

    pub fn line_height(mut self, value: f32) -> Self {
        assert!(value.is_finite() && value > 0.);
        self.typography.line_height = Some(value.min(1024.));
        self
    }

    pub fn ellipsis(mut self, max_lines: u16) -> Self {
        assert!((1..=256).contains(&max_lines));
        self.typography.max_lines = Some(max_lines);
        self.clip = true;
        self
    }

    pub fn small_font_size(mut self, v: f32) -> Self {
        self.small_font_size = Some(v);
        self
    }

    pub fn height(mut self, v: f32) -> Self {
        self.height_length = Length::Auto;
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
            blur: blur.clamp(0., 32.),
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
        self.padding_edges = None;
        self.logical_padding = None;
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
        self.width_length = Length::Auto;
        self.width = Some(v.max(1.));
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
    #[default]
    Stretch,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Justify {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlign {
    #[default]
    Start,
    Center,
    End,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextDirection {
    #[default]
    Auto,
    LeftToRight,
    RightToLeft,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorToken {
    Background,
    Surface,
    Text,
    Muted,
    Accent,
    Border,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub background: u32,
    pub surface: u32,
    pub text: u32,
    pub muted: u32,
    pub accent: u32,
    pub border: u32,
}
impl Theme {
    pub const DARK: Self = Self {
        background: 0x0b1018,
        surface: 0x141e2e,
        text: 0xf1f5ff,
        muted: 0x95a4bd,
        accent: 0xb5f36b,
        border: 0x344258,
    };
    pub const LIGHT: Self = Self {
        background: 0xf4f7fc,
        surface: 0xffffff,
        text: 0x152438,
        muted: 0x52647b,
        accent: 0x237943,
        border: 0xbac8dc,
    };
    pub fn color(self, token: ColorToken) -> u32 {
        match token {
            ColorToken::Background => self.background,
            ColorToken::Surface => self.surface,
            ColorToken::Text => self.text,
            ColorToken::Muted => self.muted,
            ColorToken::Accent => self.accent,
            ColorToken::Border => self.border,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ResponsiveStyle {
    pub min_width: f32,
    pub columns: Option<u16>,
    pub font_size: Option<f32>,
    pub padding: Option<f32>,
    pub direction: Option<Direction>,
}
impl Style {
    pub fn align(mut self, v: Align) -> Self {
        self.align = v;
        self
    }
    pub fn justify(mut self, v: Justify) -> Self {
        self.justify = v;
        self
    }
    pub fn shrink(mut self, v: f32) -> Self {
        self.shrink = v.max(0.);
        self
    }
    pub fn min_width(mut self, v: f32) -> Self {
        self.min_width = Some(v.max(0.));
        self
    }
    pub fn max_width(mut self, v: f32) -> Self {
        self.max_width = Some(v.max(1.));
        self
    }
    pub fn text_align(mut self, v: TextAlign) -> Self {
        self.text_align = v;
        self
    }
    pub fn text_direction(mut self, v: TextDirection) -> Self {
        self.text_direction = v;
        self
    }
    pub fn background_token(mut self, v: ColorToken) -> Self {
        self.background_token = Some(v);
        self
    }
    pub fn color_token(mut self, v: ColorToken) -> Self {
        self.color_token = Some(v);
        self
    }
    pub fn at_width(mut self, v: ResponsiveStyle) -> Self {
        assert!(self.queries.len() < 8 && v.min_width.is_finite());
        self.queries.push(v);
        self.queries
            .sort_by(|a, b| a.min_width.total_cmp(&b.min_width));
        self
    }
    pub fn resolve(&self, width: f32, theme: Theme) -> Self {
        let mut s = self.clone();
        if let Some(t) = s.background_token {
            s.background = Some(theme.color(t));
        }
        if let Some(t) = s.color_token {
            s.color = theme.color(t);
        }
        if width < 600. {
            if let Some(size) = s.small_font_size {
                s.font_size = size;
            }
        }
        for q in &self.queries {
            if width >= q.min_width {
                if let Some(v) = q.columns {
                    s.columns = v.max(1);
                }
                if let Some(v) = q.font_size {
                    s.font_size = v.clamp(8., 64.);
                }
                if let Some(v) = q.padding {
                    s.padding = v.max(0.);
                    s.padding_edges = None;
                    s.logical_padding = None;
                }
                if let Some(v) = q.direction {
                    s.direction = v;
                }
            }
        }
        s.queries.clear();
        s
    }
}

#[derive(Clone)]
pub struct ThemeContext(pub crate::Signal<Theme>);
