use ab_glyph::{Font, FontVec, PxScale};

#[derive(Debug, Clone)]
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

fn measure_text_width(text: &str, font: &FontVec, scale: PxScale) -> i32 {
    let mut width: f32 = 0.0;

    for c in text.chars() {
        let glyph_id = font.glyph_id(c);
        width +=
            font.h_advance_unscaled(glyph_id) * scale.x / font.units_per_em().unwrap_or(1000.0);
    }

    width.ceil() as i32
}

fn get_line_height(font: &FontVec, scale: PxScale) -> i32 {
    let ascent = font.ascent_unscaled();
    let descent = font.descent_unscaled();
    let line_gap = font.line_gap_unscaled();

    ((ascent - descent + line_gap) * scale.y / font.units_per_em().unwrap_or(1000.0)).ceil() as i32
}

fn wrap_text(
    text: &str,
    font: &FontVec,
    scale: PxScale,
    max_width: i32,
) -> Vec<Vec<(TextSegment, i32)>> {
    let mut lines = Vec::new();

    for paragraph in text.lines() {
        if paragraph.is_empty() {
            lines.push(vec![(
                TextSegment {
                    text: String::new(),
                    is_highlighted: false,
                },
                0,
            )]);
            continue;
        }

        let segments = parse_highlighted_text(paragraph);
        let mut current_line = Vec::new();
        let mut current_segment = TextSegment {
            text: String::new(),
            is_highlighted: false,
        };
        let mut line_width = 0;

        for segment in segments {
            for ch in segment.text.chars() {
                let test_char = ch.to_string();
                let char_width = measure_text_width(&test_char, font, scale);

                if line_width + char_width <= max_width {
                    if current_segment.is_highlighted == segment.is_highlighted {
                        current_segment.text.push(ch);
                    } else {
                        if !current_segment.text.is_empty() {
                            let seg_width = measure_text_width(&current_segment.text, font, scale);
                            current_line.push((current_segment.clone(), seg_width));
                        }
                        current_segment = TextSegment {
                            text: ch.to_string(),
                            is_highlighted: segment.is_highlighted,
                        };
                    }
                    line_width += char_width;
                } else {
                    if !current_segment.text.is_empty() {
                        let seg_width = measure_text_width(&current_segment.text, font, scale);
                        current_line.push((current_segment.clone(), seg_width));
                    }
                    if !current_line.is_empty() {
                        lines.push(current_line.clone());
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
            let seg_width = measure_text_width(&current_segment.text, font, scale);
            current_line.push((current_segment, seg_width));
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }

    if lines.is_empty() {
        lines.push(vec![(
            TextSegment {
                text: String::new(),
                is_highlighted: false,
            },
            0,
        )]);
    }

    lines
}

pub struct PreparedTextarea {
    pub font_size: u32,
    pub lines: Vec<Vec<(TextSegment, i32)>>,
    pub spaced_line_height: i32,
    pub block_height: i32,
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

    let mut lo = 1u32;
    let mut hi = max_size;
    let mut best_size = 1u32;
    let mut best_lines = vec![vec![(
        TextSegment {
            text: text.to_string(),
            is_highlighted: false,
        },
        0,
    )]];
    let mut best_spaced_line_height = 1i32;
    let mut best_block_height = 1i32;

    while lo <= hi {
        let mid = u32::midpoint(lo, hi);
        let scale = PxScale::from(mid as f32);
        let lines = wrap_text(text, font, scale, region_width as i32);

        let line_height = get_line_height(font, scale);
        let spaced_line_height = (line_height as f32 * (1.0 + line_spacing)).ceil() as i32;

        let mut max_width = 0;
        for line in &lines {
            let mut line_width = 0;
            for (_, width) in line {
                line_width += width;
            }
            max_width = max_width.max(line_width);
        }

        let total_height = if lines.is_empty() {
            line_height
        } else {
            spaced_line_height * lines.len() as i32
        };

        if max_width <= region_width as i32 && total_height <= region_height as i32 {
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
