use kawi_buffer::Buffer;

use super::Fold;

struct OpenFold {
    base: usize,
    start_index: usize,
    start_row: usize,
    depth: usize,
}

pub(super) fn folds(buffer: &Buffer, shiftwidth: usize, tabstop: usize) -> Vec<Fold> {
    let shiftwidth = shiftwidth.max(1);
    let tabstop = tabstop.max(1);
    let lines = indented_lines(buffer, shiftwidth, tabstop);
    let mut folds = Vec::new();
    let mut open: Vec<OpenFold> = Vec::new();
    let mut previous = None;
    let mut previous_depth = 0;

    for (index, (row, depth)) in lines.iter().copied().enumerate() {
        while open.last().is_some_and(|fold| depth <= fold.base) {
            close_fold(&mut folds, open.pop(), previous);
        }
        if depth > previous_depth {
            open.push(OpenFold {
                base: previous_depth,
                start_index: index,
                start_row: row,
                depth,
            });
        }
        previous = Some((index, row));
        previous_depth = depth;
    }

    while let Some(fold) = open.pop() {
        close_fold(&mut folds, Some(fold), previous);
    }

    folds.sort_by_key(|fold| (fold.start, fold.end));
    folds
}

fn indented_lines(buffer: &Buffer, shiftwidth: usize, tabstop: usize) -> Vec<(usize, usize)> {
    (0..buffer.line_count())
        .filter_map(|row| {
            let text = buffer.line(row)?;
            if text.trim().is_empty() {
                return None;
            }
            Some((row, indent_width(&text, tabstop) / shiftwidth))
        })
        .collect()
}

fn close_fold(folds: &mut Vec<Fold>, open: Option<OpenFold>, previous: Option<(usize, usize)>) {
    let (Some(fold), Some((end_index, end_row))) = (open, previous) else {
        return;
    };
    if end_index > fold.start_index {
        folds.push(Fold {
            start: fold.start_row,
            end: end_row,
            depth: fold.depth,
        });
    }
}

fn indent_width(text: &str, tabstop: usize) -> usize {
    let mut width: usize = 0;
    for ch in text.chars() {
        match ch {
            ' ' => width += 1,
            '\t' => width = width.saturating_add(tabstop - width % tabstop),
            _ => break,
        }
    }
    width
}
