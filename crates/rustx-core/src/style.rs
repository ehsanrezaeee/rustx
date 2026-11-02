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
