use super::*;

pub fn report_err<T, E: Display>(result: Result<T, E>, msg: impl AsRef<str>) -> Result<T, ()> {
    match result {
        Err(err) => {
            error!("{}: {err}", msg.as_ref());
            Err(())
        }
        Ok(value) => Ok(value),
    }
}

pub fn report_warn<T, E: Display>(result: Result<T, E>, msg: impl AsRef<str>) -> Result<T, ()> {
    match result {
        Err(err) => {
            warn!("{}: {err}", msg.as_ref());
            Err(())
        }
        Ok(value) => Ok(value),
    }
}

pub fn aabb_outline(aabb: Aabb2<f32>) -> Chain<f32> {
    let [a, b, c, d] = aabb.corners();
    Chain::new(vec![(a + b) / 2.0, a, d, c, b, (a + b) / 2.0])
}

pub fn fit_text(text: impl AsRef<str>, font: impl AsRef<geng::Font>, target: Aabb2<f32>) -> f32 {
    // TODO: check height
    target.width()
        / font
            .as_ref()
            .measure_bounding_box(
                text.as_ref(),
                vec2(geng::TextAlign::LEFT, geng::TextAlign::LEFT),
            )
            .unwrap()
            .width()
}

pub fn split_text_lines(
    text: impl AsRef<str>,
    font: impl AsRef<geng::Font>,
    size: f32,
    target_width: f32,
) -> Vec<String> {
    let font = font.as_ref();
    let mut lines = Vec::new();
    let mut line = String::new();

    let measure = |str: &str| {
        font.measure_bounding_box(str, vec2(geng::TextAlign::LEFT, geng::TextAlign::LEFT))
            .unwrap_or(Aabb2::ZERO)
            .width()
            * size
    };

    for word in text.as_ref().split_whitespace() {
        if line.is_empty() {
            line.push_str(word);
        } else {
            let width = measure(&line);
            if width + measure(" ") + measure(word) < target_width {
                // Word fits in the line
                line.push(' ');
                line.push_str(word);
            } else {
                // Start new line
                let mut new_line = String::new();
                std::mem::swap(&mut new_line, &mut line);
                lines.push(new_line);
                line = word.to_owned();
            }
        }
    }
    lines.push(line);
    lines
}
