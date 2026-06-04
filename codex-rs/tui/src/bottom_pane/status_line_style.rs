//! Theme-derived styling for the configurable footer statusline.

use ratatui::prelude::Stylize;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;

use super::status_line_setup::StatusLineItem;
use crate::render::highlight::foreground_style_for_scopes;

const STATUS_LINE_SEPARATOR: &str = " · ";
const HUD_SEPARATOR: &str = " │ ";
const HUD_BAR_EMPTY: char = '─';
const STATUS_LINE_COLOR_SATURATION_PERCENT: u16 = 85;
const STATUS_LINE_COLOR_BRIGHTNESS_PERCENT: u16 = 100;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StatusLineAccent {
    Model,
    Path,
    Branch,
    State,
    Usage,
    Limit,
    Metadata,
    Mode,
    Thread,
    Progress,
}

impl StatusLineAccent {
    fn for_item(item: StatusLineItem) -> Self {
        match item {
            StatusLineItem::ModelName
            | StatusLineItem::ModelWithReasoning
            | StatusLineItem::Reasoning => Self::Model,
            StatusLineItem::CurrentDir | StatusLineItem::ProjectRoot => Self::Path,
            StatusLineItem::GitBranch
            | StatusLineItem::PullRequestNumber
            | StatusLineItem::BranchChanges => Self::Branch,
            StatusLineItem::Status => Self::State,
            StatusLineItem::ContextRemaining
            | StatusLineItem::ContextUsed
            | StatusLineItem::ContextWindowSize
            | StatusLineItem::UsedTokens
            | StatusLineItem::TotalInputTokens
            | StatusLineItem::TotalOutputTokens => Self::Usage,
            StatusLineItem::FiveHourLimit | StatusLineItem::WeeklyLimit => Self::Limit,
            StatusLineItem::CodexVersion | StatusLineItem::SessionId => Self::Metadata,
            StatusLineItem::FastMode | StatusLineItem::RawOutput | StatusLineItem::CustomLine => {
                Self::Mode
            }
            StatusLineItem::Permissions => Self::Mode,
            StatusLineItem::ApprovalMode => Self::Mode,
            StatusLineItem::ThreadTitle => Self::Thread,
            StatusLineItem::TaskProgress => Self::Progress,
        }
    }

    fn scopes(self) -> &'static [&'static str] {
        match self {
            Self::Model => &["entity.name.type", "support.type", "variable"],
            Self::Path => &["string", "markup.underline.link"],
            Self::Branch => &["entity.name.function", "entity.name.tag"],
            Self::State => &["keyword.control", "keyword"],
            Self::Usage => &["constant.numeric", "constant"],
            Self::Limit => &["constant.language", "storage.type"],
            Self::Metadata => &["comment", "constant.other"],
            Self::Mode => &["storage.modifier", "keyword.operator"],
            Self::Thread => &["markup.heading", "entity.name.section"],
            Self::Progress => &["markup.inserted", "constant.numeric"],
        }
    }

    fn fallback_style(self) -> Style {
        match self {
            Self::Model | Self::State | Self::Metadata | Self::Mode => Style::default().cyan(),
            Self::Path | Self::Usage | Self::Progress => Style::default().green(),
            Self::Branch | Self::Limit | Self::Thread => Style::default().magenta(),
        }
    }
}

pub(crate) fn status_line_from_segments<I>(
    segments: I,
    use_theme_colors: bool,
) -> Option<Vec<Line<'static>>>
where
    I: IntoIterator<Item = (StatusLineItem, String)>,
{
    status_line_from_segments_with_resolver(segments, use_theme_colors, |accent| {
        foreground_style_for_scopes(accent.scopes())
    })
}

fn status_line_from_segments_with_resolver<I, F>(
    segments: I,
    use_theme_colors: bool,
    theme_style_for_accent: F,
) -> Option<Vec<Line<'static>>>
where
    I: IntoIterator<Item = (StatusLineItem, String)>,
    F: Fn(StatusLineAccent) -> Option<Style>,
{
    let segments: Vec<(StatusLineItem, String)> = segments
        .into_iter()
        .filter(|(_, text)| !text.is_empty())
        .collect();
    if segments.is_empty() {
        return None;
    }

    if should_use_hud_layout(&segments) {
        return Some(status_hud_lines(&segments, use_theme_colors));
    }

    Some(vec![status_line_legacy_from_segments(
        segments,
        use_theme_colors,
        theme_style_for_accent,
    )])
}

fn status_line_legacy_from_segments<I, F>(
    segments: I,
    use_theme_colors: bool,
    theme_style_for_accent: F,
) -> Line<'static>
where
    I: IntoIterator<Item = (StatusLineItem, String)>,
    F: Fn(StatusLineAccent) -> Option<Style>,
{
    let mut spans = Vec::new();
    for (item, text) in segments {
        if !spans.is_empty() {
            spans.push(STATUS_LINE_SEPARATOR.dim());
        }
        let style = if use_theme_colors {
            let accent = StatusLineAccent::for_item(item);
            soften_status_line_style(
                theme_style_for_accent(accent).unwrap_or_else(|| accent.fallback_style()),
            )
        } else {
            Style::default().dim()
        };
        let style = if item == StatusLineItem::PullRequestNumber {
            style.underlined()
        } else {
            style
        };
        spans.push(Span::styled(text, style));
    }

    Line::from(spans)
}

fn should_use_hud_layout(segments: &[(StatusLineItem, String)]) -> bool {
    let has_identity = segments.iter().any(|(item, _)| {
        matches!(
            item,
            StatusLineItem::ModelName | StatusLineItem::ModelWithReasoning
        )
    });
    let has_meter = segments.iter().any(|(item, _)| {
        matches!(
            item,
            StatusLineItem::ContextRemaining
                | StatusLineItem::ContextUsed
                | StatusLineItem::FiveHourLimit
                | StatusLineItem::WeeklyLimit
        )
    });

    has_identity && has_meter
}

fn status_hud_lines(
    segments: &[(StatusLineItem, String)],
    use_theme_colors: bool,
) -> Vec<Line<'static>> {
    let mut identity_spans = Vec::new();
    let mut meter_spans = Vec::new();
    let mut detail_spans = Vec::new();

    for (item, text) in segments {
        match item {
            StatusLineItem::ContextRemaining
            | StatusLineItem::ContextUsed
            | StatusLineItem::FiveHourLimit
            | StatusLineItem::WeeklyLimit => {
                push_hud_separator(&mut meter_spans, use_theme_colors);
                meter_spans.extend(hud_meter_spans(*item, text, use_theme_colors));
            }
            StatusLineItem::Status
            | StatusLineItem::Permissions
            | StatusLineItem::ApprovalMode
            | StatusLineItem::FastMode
            | StatusLineItem::RawOutput
            | StatusLineItem::CustomLine
            | StatusLineItem::TaskProgress => {
                push_hud_separator(&mut detail_spans, use_theme_colors);
                detail_spans.extend(hud_segment_spans(*item, text, use_theme_colors));
            }
            _ => {
                push_hud_separator(&mut identity_spans, use_theme_colors);
                identity_spans.extend(hud_segment_spans(*item, text, use_theme_colors));
            }
        }
    }

    [identity_spans, meter_spans, detail_spans]
        .into_iter()
        .filter(|spans| !spans.is_empty())
        .map(Line::from)
        .collect()
}

fn push_hud_separator(spans: &mut Vec<Span<'static>>, use_colors: bool) {
    if !spans.is_empty() {
        spans.push(Span::styled(HUD_SEPARATOR, hud_separator_style(use_colors)));
    }
}

fn hud_segment_spans(item: StatusLineItem, text: &str, use_colors: bool) -> Vec<Span<'static>> {
    if !use_colors {
        return vec![Span::styled(text.to_string(), Style::default().dim())];
    }

    match item {
        StatusLineItem::ModelName | StatusLineItem::ModelWithReasoning => vec![
            Span::styled("[", hud_dim_style()),
            Span::styled(text.to_string(), hud_model_style()),
            Span::styled("]", hud_dim_style()),
        ],
        StatusLineItem::ProjectRoot | StatusLineItem::CurrentDir => {
            vec![Span::styled(text.to_string(), hud_project_style())]
        }
        StatusLineItem::GitBranch => vec![
            Span::styled("git:(", hud_dim_style()),
            Span::styled(text.to_string(), hud_branch_style()),
            Span::styled(")", hud_dim_style()),
        ],
        StatusLineItem::UsedTokens => vec![Span::styled(text.to_string(), hud_token_style())],
        StatusLineItem::ThreadTitle | StatusLineItem::CodexVersion | StatusLineItem::SessionId => {
            vec![Span::styled(text.to_string(), hud_secondary_style())]
        }
        StatusLineItem::TaskProgress => vec![Span::styled(text.to_string(), hud_mint_style())],
        _ => vec![Span::styled(text.to_string(), hud_secondary_style())],
    }
}

fn hud_meter_spans(item: StatusLineItem, text: &str, use_colors: bool) -> Vec<Span<'static>> {
    if !use_colors {
        return vec![Span::styled(text.to_string(), Style::default().dim())];
    }

    let parts = text.split_whitespace().collect::<Vec<_>>();
    let Some(percent_index) = parts
        .iter()
        .position(|part| hud_meter_percent_value(part).is_some())
    else {
        return vec![Span::styled(text.to_string(), hud_secondary_style())];
    };
    if percent_index < 2 {
        return vec![Span::styled(text.to_string(), hud_secondary_style())];
    }
    let label = parts[..percent_index - 1].join(" ");
    let bar = parts[percent_index - 1];
    let percent = parts[percent_index];
    let suffix = (percent_index + 1 < parts.len()).then(|| parts[percent_index + 1..].join(" "));

    let (filled_style, label_style) = match item {
        StatusLineItem::ContextRemaining | StatusLineItem::ContextUsed => {
            (hud_mint_style(), hud_label_style())
        }
        StatusLineItem::FiveHourLimit | StatusLineItem::WeeklyLimit => {
            (hud_rate_style(), hud_label_style())
        }
        _ => (hud_secondary_style(), hud_label_style()),
    };
    let filled_style = hud_meter_threshold_style(item, filled_style, percent);
    let empty_style = hud_bar_shadow_style(filled_style);

    let mut spans = vec![
        Span::styled(label.to_string(), label_style),
        Span::styled(" ", hud_dim_style()),
    ];
    for ch in bar.chars() {
        let style = if ch == HUD_BAR_EMPTY {
            empty_style
        } else {
            filled_style
        };
        spans.push(Span::styled(ch.to_string(), style));
    }
    spans.push(Span::styled(" ", hud_dim_style()));
    spans.push(Span::styled(percent.to_string(), filled_style));
    if let Some(suffix) = suffix {
        spans.push(Span::styled(format!(" {suffix}"), hud_secondary_style()));
    }
    spans
}

fn hud_meter_threshold_style(item: StatusLineItem, default_style: Style, percent: &str) -> Style {
    let Some(value) = hud_meter_percent_value(percent) else {
        return default_style;
    };

    match item {
        StatusLineItem::ContextUsed => {
            if value > 80 {
                hud_orange_style()
            } else if value > 60 {
                hud_yellow_style()
            } else {
                default_style
            }
        }
        StatusLineItem::ContextRemaining
        | StatusLineItem::FiveHourLimit
        | StatusLineItem::WeeklyLimit => {
            if value <= 20 {
                hud_orange_style()
            } else if value <= 40 {
                hud_yellow_style()
            } else {
                default_style
            }
        }
        StatusLineItem::ModelName
        | StatusLineItem::ModelWithReasoning
        | StatusLineItem::Reasoning
        | StatusLineItem::ProjectRoot
        | StatusLineItem::CurrentDir
        | StatusLineItem::GitBranch
        | StatusLineItem::PullRequestNumber
        | StatusLineItem::BranchChanges
        | StatusLineItem::Status
        | StatusLineItem::Permissions
        | StatusLineItem::ApprovalMode
        | StatusLineItem::UsedTokens
        | StatusLineItem::ContextWindowSize
        | StatusLineItem::TotalInputTokens
        | StatusLineItem::TotalOutputTokens
        | StatusLineItem::CodexVersion
        | StatusLineItem::FastMode
        | StatusLineItem::RawOutput
        | StatusLineItem::CustomLine
        | StatusLineItem::ThreadTitle
        | StatusLineItem::SessionId
        | StatusLineItem::TaskProgress => default_style,
    }
}

fn hud_meter_percent_value(percent: &str) -> Option<i64> {
    percent.strip_suffix('%')?.parse().ok()
}

fn hud_separator_style(use_colors: bool) -> Style {
    if use_colors {
        hud_dim_style()
    } else {
        Style::default().dim()
    }
}

fn hud_model_style() -> Style {
    Style::default().fg(Color::LightBlue).bold()
}

fn hud_project_style() -> Style {
    Style::default().fg(Color::LightGreen).bold()
}

fn hud_branch_style() -> Style {
    Style::default().fg(Color::LightCyan).bold()
}

fn hud_token_style() -> Style {
    Style::default().fg(Color::LightYellow).bold()
}

fn hud_mint_style() -> Style {
    Style::default().fg(Color::LightGreen).bold()
}

fn hud_rate_style() -> Style {
    Style::default().fg(Color::LightBlue).bold()
}

fn hud_yellow_style() -> Style {
    Style::default().fg(Color::Yellow).bold()
}

fn hud_orange_style() -> Style {
    Style::default().fg(Color::LightRed).bold()
}

fn hud_bar_shadow_style(_filled_style: Style) -> Style {
    Style::default().fg(Color::DarkGray)
}

fn hud_label_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

fn hud_secondary_style() -> Style {
    Style::default().fg(Color::Gray)
}

fn hud_dim_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

fn soften_status_line_style(mut style: Style) -> Style {
    if let Some(fg) = style.fg {
        style.fg = Some(soften_status_line_color(fg));
    }
    style
}

#[allow(clippy::disallowed_methods)]
fn soften_status_line_color(color: Color) -> Color {
    match color {
        Color::Rgb(r, g, b) => {
            let luma = weighted_luma(r, g, b);
            Color::Rgb(
                soften_rgb_channel(r, luma),
                soften_rgb_channel(g, luma),
                soften_rgb_channel(b, luma),
            )
        }
        Color::LightRed => Color::Red,
        Color::LightGreen => Color::Green,
        Color::LightYellow => Color::Yellow,
        Color::LightBlue => Color::Blue,
        Color::LightMagenta => Color::Magenta,
        Color::LightCyan => Color::Cyan,
        Color::White => Color::Gray,
        Color::Reset
        | Color::Black
        | Color::Red
        | Color::Green
        | Color::Yellow
        | Color::Blue
        | Color::Magenta
        | Color::Cyan
        | Color::Gray
        | Color::DarkGray
        | Color::Indexed(_) => color,
    }
}

fn weighted_luma(r: u8, g: u8, b: u8) -> u16 {
    (77 * u16::from(r) + 150 * u16::from(g) + 29 * u16::from(b)) / 256
}

fn soften_rgb_channel(channel: u8, luma: u16) -> u8 {
    let channel = u16::from(channel);
    let softened = (channel * STATUS_LINE_COLOR_SATURATION_PERCENT
        + luma * (100 - STATUS_LINE_COLOR_SATURATION_PERCENT)
        + 50)
        / 100;

    ((softened * STATUS_LINE_COLOR_BRIGHTNESS_PERCENT + 50) / 100) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use ratatui::style::Modifier;

    fn line_text(line: &Line<'static>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>()
    }

    fn only_line(lines: Vec<Line<'static>>) -> Line<'static> {
        assert_eq!(lines.len(), 1);
        lines.into_iter().next().expect("one line")
    }

    #[test]
    fn status_line_segments_preserve_order_and_plain_text() {
        let line = only_line(
            status_line_from_segments_with_resolver(
                [
                    (StatusLineItem::ModelName, "gpt-5".to_string()),
                    (StatusLineItem::CurrentDir, "/repo".to_string()),
                    (StatusLineItem::GitBranch, "main".to_string()),
                ],
                /*use_theme_colors*/ true,
                |_| None,
            )
            .expect("status line"),
        );

        assert_eq!(line_text(&line), "gpt-5 · /repo · main");
        assert_eq!(line.spans[0].style.fg, Some(Color::Cyan));
        assert!(!line.spans[0].style.add_modifier.contains(Modifier::DIM));
        assert_eq!(line.spans[2].style.fg, Some(Color::Green));
        assert!(!line.spans[2].style.add_modifier.contains(Modifier::DIM));
        assert_eq!(line.spans[4].style.fg, Some(Color::Magenta));
        assert!(!line.spans[4].style.add_modifier.contains(Modifier::DIM));
    }

    #[test]
    fn status_line_segments_dim_separators_and_use_theme_styles_first() {
        let line = only_line(
            status_line_from_segments_with_resolver(
                [
                    (StatusLineItem::ModelName, "gpt-5".to_string()),
                    (StatusLineItem::UsedTokens, "12 used".to_string()),
                ],
                /*use_theme_colors*/ true,
                |accent| match accent {
                    StatusLineAccent::Model => Some(Style::default().red()),
                    _ => None,
                },
            )
            .expect("status line"),
        );

        assert_eq!(line.spans[0].style.fg, Some(Color::Red));
        assert!(!line.spans[0].style.add_modifier.contains(Modifier::DIM));
        assert!(line.spans[1].style.add_modifier.contains(Modifier::DIM));
        assert_eq!(line.spans[2].style.fg, Some(Color::Green));
        assert!(!line.spans[2].style.add_modifier.contains(Modifier::DIM));
    }

    #[test]
    #[allow(clippy::disallowed_methods)]
    fn status_line_segments_soften_rgb_theme_styles_without_dimming_text() {
        let line = only_line(
            status_line_from_segments_with_resolver(
                [(StatusLineItem::ModelName, "gpt-5".to_string())],
                /*use_theme_colors*/ true,
                |_| Some(Style::default().fg(Color::Rgb(255, 0, 0))),
            )
            .expect("status line"),
        );

        assert_eq!(line.spans[0].style.fg, Some(Color::Rgb(228, 11, 11)));
        assert!(!line.spans[0].style.add_modifier.contains(Modifier::DIM));
    }

    #[test]
    fn status_line_segments_can_disable_theme_colors() {
        let line = only_line(
            status_line_from_segments_with_resolver(
                [
                    (StatusLineItem::ModelName, "gpt-5".to_string()),
                    (StatusLineItem::UsedTokens, "12 used".to_string()),
                ],
                /*use_theme_colors*/ false,
                |_| Some(Style::default().red()),
            )
            .expect("status line"),
        );

        assert_eq!(line_text(&line), "gpt-5 · 12 used");
        assert_eq!(line.spans[0].style.fg, None);
        assert!(line.spans[0].style.add_modifier.contains(Modifier::DIM));
        assert!(line.spans[1].style.add_modifier.contains(Modifier::DIM));
        assert_eq!(line.spans[2].style.fg, None);
        assert!(line.spans[2].style.add_modifier.contains(Modifier::DIM));
    }

    #[test]
    fn pull_request_number_uses_link_style() {
        let line = only_line(
            status_line_from_segments_with_resolver(
                [(StatusLineItem::PullRequestNumber, "PR #20252".to_string())],
                /*use_theme_colors*/ false,
                |_| None,
            )
            .expect("status line"),
        );

        assert_eq!(line.spans[0].style.fg, None);
        assert!(line.spans[0].style.add_modifier.contains(Modifier::DIM));
        assert!(
            line.spans[0]
                .style
                .add_modifier
                .contains(Modifier::UNDERLINED)
        );
    }

    #[test]
    fn status_line_segments_return_none_when_empty() {
        assert_eq!(
            status_line_from_segments_with_resolver(
                Vec::<(StatusLineItem, String)>::new(),
                /*use_theme_colors*/ true,
                |_| None,
            ),
            None
        );
    }

    #[test]
    fn hud_layout_groups_identity_meters_and_detail_rows() {
        let lines = status_line_from_segments_with_resolver(
            [
                (
                    StatusLineItem::ModelWithReasoning,
                    "gpt-5.5 xhigh".to_string(),
                ),
                (StatusLineItem::ProjectRoot, "codex".to_string()),
                (StatusLineItem::GitBranch, "adit/chud* ↑2".to_string()),
                (
                    StatusLineItem::ContextUsed,
                    "Context ━───────── 64%".to_string(),
                ),
                (
                    StatusLineItem::FiveHourLimit,
                    "secondary usage ━━━━━━━━━─ 85% left".to_string(),
                ),
                (StatusLineItem::Permissions, "Workspace".to_string()),
                (StatusLineItem::ApprovalMode, "on-request".to_string()),
                (StatusLineItem::TaskProgress, "Tasks 2/5".to_string()),
            ],
            /*use_theme_colors*/ true,
            |_| None,
        )
        .expect("hud lines");

        assert_eq!(lines.len(), 3);
        assert_eq!(
            line_text(&lines[0]),
            "[gpt-5.5 xhigh] │ codex │ git:(adit/chud* ↑2)"
        );
        assert_eq!(
            line_text(&lines[1]),
            "Context ━───────── 64% │ secondary usage ━━━━━━━━━─ 85% left"
        );
        assert_eq!(
            lines[1]
                .spans
                .iter()
                .find(|span| span.content.as_ref() == "64%")
                .expect("used percent")
                .style
                .fg,
            Some(Color::Yellow)
        );
        assert_eq!(line_text(&lines[2]), "Workspace │ on-request │ Tasks 2/5");
        assert_eq!(
            lines[1]
                .spans
                .iter()
                .find(|span| span.content.as_ref() == "85%")
                .expect("remaining percent")
                .style
                .fg,
            Some(Color::LightBlue)
        );
    }
}
