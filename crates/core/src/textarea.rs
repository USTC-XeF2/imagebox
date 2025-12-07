use ab_glyph::{Font, FontVec, PxScale, PxScaleFont, ScaleFont};

#[derive(Debug, Clone, Default)]
pub struct TextSegment {
    pub text: String,
    pub is_highlighted: bool,
}

fn parse_highlighted_text(text: &str) -> Vec<TextSegment> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut in_highlight = false;

    for ch in text.chars() {
        if ch == '【' || ch == '[' {
            if !current.is_empty() {
                segments.push(TextSegment {
                    text: current.clone(),
                    is_highlighted: in_highlight,
                });
                current.clear();
            }
            in_highlight = true;
            current.push(ch);
        } else if ch == '】' || ch == ']' {
            current.push(ch);
            if in_highlight {
                segments.push(TextSegment {
                    text: current.clone(),
                    is_highlighted: true,
                });
                current.clear();
                in_highlight = false;
            }
        } else {
            current.push(ch);
        }
    }

    if !current.is_empty() {
        segments.push(TextSegment {
            text: current,
            is_highlighted: in_highlight,
        });
    }

    segments
}

pub fn get_scaled_font(font: &FontVec, font_size: u32) -> PxScaleFont<&FontVec> {
    let height_unscaled = font.height_unscaled();
    let units_per_em = font.units_per_em().unwrap_or(1000.0);

    let corr_font_size = font_size as f32 * height_unscaled / units_per_em;

    font.as_scaled(PxScale {
        x: corr_font_size,
        y: corr_font_size,
    })
}

fn measure_text_width(text: &str, scaled_font: PxScaleFont<&FontVec>) -> u32 {
    text.chars().fold(0, |acc, c| {
        let glyph_id = scaled_font.glyph_id(c);
        acc + scaled_font.h_advance(glyph_id).ceil() as u32
    })
}

fn wrap_text(
    text: &str,
    scaled_font: PxScaleFont<&FontVec>,
    max_width: u32,
) -> Vec<Vec<(TextSegment, u32)>> {
    let mut lines = Vec::new();

    for paragraph in text.lines() {
        if paragraph.is_empty() {
            lines.push(vec![(TextSegment::default(), 0)]);
            continue;
        }

        let segments = parse_highlighted_text(paragraph);
        let mut current_line = Vec::new();
        let mut current_segment = TextSegment::default();
        let mut line_width = 0;

        for segment in segments {
            for ch in segment.text.chars() {
                let test_char = ch.to_string();
                let char_width = measure_text_width(&test_char, scaled_font);

                if line_width + char_width <= max_width {
                    if current_segment.is_highlighted == segment.is_highlighted {
                        current_segment.text.push(ch);
                    } else {
                        if !current_segment.text.is_empty() {
                            let seg_width = measure_text_width(&current_segment.text, scaled_font);
                            current_line.push((current_segment, seg_width));
                        }
                        current_segment = TextSegment {
                            text: ch.to_string(),
                            is_highlighted: segment.is_highlighted,
                        };
                    }
                    line_width += char_width;
                } else {
                    if !current_segment.text.is_empty() {
                        let seg_width = measure_text_width(&current_segment.text, scaled_font);
                        current_line.push((current_segment, seg_width));
                    }
                    if !current_line.is_empty() {
                        lines.push(current_line);
                    }
                    current_line = Vec::new();
                    current_segment = TextSegment {
                        text: ch.to_string(),
                        is_highlighted: segment.is_highlighted,
                    };
                    line_width = char_width;
                }
            }
        }

        if !current_segment.text.is_empty() {
            let seg_width = measure_text_width(&current_segment.text, scaled_font);
            current_line.push((current_segment, seg_width));
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }

    if lines.is_empty() {
        lines.push(vec![(TextSegment::default(), 0)]);
    }

    lines
}

pub struct PreparedTextarea {
    pub font_size: u32,
    pub lines: Vec<Vec<(TextSegment, u32)>>,
    pub spaced_line_height: u32,
    pub block_height: u32,
}

pub fn prepare_textarea(
    text: &str,
    font: &FontVec,
    region_width: u32,
    region_height: u32,
    max_font_size: Option<u32>,
    line_spacing: f32,
) -> PreparedTextarea {
    let max_size = if let Some(max_h) = max_font_size {
        max_h.min(region_height)
    } else {
        region_height
    };

    let mut lo = 1;
    let mut hi = max_size;
    let mut best_size = 1;
    let mut best_lines = vec![vec![(
        TextSegment {
            text: text.to_string(),
            is_highlighted: false,
        },
        0,
    )]];
    let mut best_spaced_line_height = 1;
    let mut best_block_height = 1;

    while lo <= hi {
        let mid = u32::midpoint(lo, hi);
        let scaled_font = get_scaled_font(font, mid);
        let lines = wrap_text(text, scaled_font, region_width);

        let line_height = scaled_font.height();
        let spaced_line_height = (line_height * (1.0 + line_spacing)).ceil() as u32;

        let max_width = lines
            .iter()
            .map(|line| line.iter().map(|(_, width)| width).sum())
            .max()
            .unwrap_or(0);

        let total_height =
            spaced_line_height * lines.len() as u32 - (line_height * line_spacing).ceil() as u32;

        if max_width <= region_width && total_height <= region_height {
            best_size = mid;
            best_lines = lines;
            best_spaced_line_height = spaced_line_height;
            best_block_height = total_height;
            lo = mid + 1;
        } else {
            hi = mid - 1;
        }
    }

    PreparedTextarea {
        font_size: best_size,
        lines: best_lines,
        spaced_line_height: best_spaced_line_height,
        block_height: best_block_height,
    }
}
