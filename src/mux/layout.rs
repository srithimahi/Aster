use super::pane::Pane;

#[derive(Clone, Copy, Debug)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug)]
pub enum FocusDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug)]
pub struct PaneRect {
    pub pane_id: usize,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub enum PaneNode {
    Pane(Pane),

    Split {
        direction: SplitDirection,
        ratio: f32,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}

impl PaneNode {
    pub fn new_pane(
        columns: usize,
        rows: usize,
    ) -> Self {
        Self::Pane(
            Pane::new(
                columns,
                rows,
            )
        )
    }

    pub fn process_output(
        &mut self,
    ) -> bool {
        match self {
            PaneNode::Pane(pane) => {
                pane.process_output()
            }

            PaneNode::Split {
                first,
                second,
                ..
            } => {
                let first_received = first.process_output();
                let second_received = second.process_output();
                first_received || second_received
            }
        }
    }

    pub fn resize(
        &mut self,
        columns: usize,
        rows: usize,
    ) {
        match self {
            PaneNode::Pane(pane) => {
                pane.resize(
                    columns, 
                    rows,
                );
            }

            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                match direction {
                    SplitDirection::Vertical => {
                        let first_columns = ((*ratio * columns as f32) as usize)
                            .clamp(
                                1,
                                columns.saturating_sub(1).max(1),
                            );

                        let second_columns = columns.saturating_sub(
                            first_columns
                        ).max(1);

                        first.resize(
                            first_columns,
                            rows,
                        );

                        second.resize(
                            second_columns,
                            rows,
                        );
                    }

                    SplitDirection::Horizontal => {
                        let first_rows = 
                            ((*ratio * rows as f32) as usize)
                            .clamp(
                                1,
                                rows.saturating_sub(1).max(1),
                            );
                        
                        let second_rows = 
                            rows.saturating_sub(
                                first_rows
                            ).max(1);

                        first.resize(
                            columns,
                            first_rows,
                        );

                        second.resize(
                            columns,
                            second_rows,
                        );
                    }
                }
            }
        }
    }

    pub fn first_pane(
        &self,
    ) -> &Pane {
        match self {
            PaneNode::Pane(pane) => {
                pane
            }

            PaneNode::Split {
                first,
                ..
            } => {
                first.first_pane()
            }
        }
    }

    pub fn first_pane_mut(
        &mut self,
    ) -> &mut Pane {
        match self {
            PaneNode::Pane(pane) => {
                pane
            }

            PaneNode::Split {
                first,
                ..
            } => {
                first.first_pane_mut()
            }
        }
    }

    pub fn split(
        &mut self,
        direction: SplitDirection,
        columns: usize,
        rows: usize,
    ) {
        let replacement = PaneNode::new_pane(
            columns,
            rows,
        );

        let original = std::mem::replace(
            self,
            replacement,
        );

        *self = PaneNode::Split {
            direction,
            ratio: 0.5,
            first: Box::new(
                original
            ),

            second: Box::new(
                PaneNode::new_pane(
                    columns,
                    rows,
                )
            ),
        };

        self.resize(
            columns,
            rows,
        );
    }

    pub fn for_each_pane<F>(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        callback: &mut F,
    )
    where 
        F: FnMut(
            &Pane,
            u32,
            u32,
            u32,
            u32,
        ),
    {
        match self {
            PaneNode::Pane(pane) => {
                callback(
                    pane,
                    x,
                    y,
                    width,
                    height,
                );
            }

            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                match direction {
                    SplitDirection::Vertical => {
                        let first_width = (width as f32 * *ratio) as u32;
                        let second_width = width.saturating_sub(first_width);

                        first.for_each_pane(
                            x,
                            y,
                            first_width,
                            height,
                            callback,
                        );

                        second.for_each_pane(
                            x + first_width,
                            y,
                            second_width,
                            height,
                            callback,
                        );
                    }

                    SplitDirection::Horizontal => {
                        let first_height = (height as f32 * *ratio) as u32;

                        let second_height = height.saturating_sub(first_height);
                        
                        first.for_each_pane(
                            x,
                            y,
                            width,
                            first_height,
                            callback,
                        );

                        second.for_each_pane(
                            x,
                            y + first_height,
                            width,
                            second_height,
                            callback,
                        );
                    }
                }
            }
        }
    }

    pub fn find_pane(
        &self,
        pane_id: usize,
    ) -> Option<&Pane> {
        match self {
            PaneNode::Pane(pane) => {
                if pane.id() == pane_id {
                    Some(pane)
                } else {
                    None
                }
            }

            PaneNode::Split {
                first,
                second,
                ..
            } => {
                first.find_pane(pane_id).or_else(|| {
                    second.find_pane(pane_id)
                })
            }
        }
    }

    pub fn find_pane_mut(
        &mut self,
        pane_id: usize,
    ) -> Option<&mut Pane> {
        match self {
            PaneNode::Pane(pane) => {
                if pane.id() == pane_id {
                    Some(pane)
                } else {
                    None
                }
            }

            PaneNode::Split {
                first,
                second,
                ..
            } => {
                if let Some(pane) = first.find_pane_mut(pane_id)
                {
                    return Some(pane);
                }

                second.find_pane_mut(pane_id)
            }
        }
    }

    pub fn split_pane(
        &mut self,
        pane_id: usize,
        direction: SplitDirection,
        columns: usize,
        rows: usize,
    ) -> Option<usize> {
        match self {
            PaneNode::Pane(pane) => {
                if pane.id() != pane_id {
                    return None;
                }

                let new_pane = Pane::new(columns, rows);
                let new_pane_id = new_pane.id();
                let replacement = PaneNode::Pane(new_pane);
                let original = std::mem::replace(
                    self,
                    replacement,
                );

                let second = std::mem::replace(
                    self,
                    PaneNode::new_pane(
                        columns,
                        rows,
                    ),
                );

                *self = PaneNode::Split {
                    direction,
                    ratio: 0.5,
                    first: Box::new(original),
                    second: Box::new(second),
                };

                self.resize(
                    columns,
                    rows,
                );

                Some(new_pane_id)
            }

            PaneNode::Split {
                first,
                second,
                ..
            } => {
                if let Some(new_id) = 
                    first.split_pane(
                        pane_id,
                        direction,
                        columns,
                        rows,
                    )
                {
                    return Some(new_id);
                }

                second.split_pane(
                    pane_id,
                    direction,
                    columns,
                    rows,
                )
            }
        }
    }

    pub fn collect_pane_ids(
        &self,
        ids: &mut Vec<usize>,
    ) {
        match self {
            PaneNode::Pane(pane) => {
                ids.push(pane.id());
            }

            PaneNode::Split {
                first,
                second,
                ..
            } => {
                first.collect_pane_ids(ids);
                second.collect_pane_ids(ids);
            }
        }
    }

    pub fn remove_pane(
        &mut self,
        pane_id: usize,
    ) -> bool {
        match self {
            PaneNode::Pane(_) => false,
            PaneNode::Split {
                first,
                second,
                ..
            } => {
                let first_is_target = 
                    matches!(
                        first.as_ref(),
                        PaneNode::Pane(pane)
                            if pane.id() == pane_id
                    );

                if first_is_target {
                    let replacement = 
                        std::mem::replace(
                            second.as_mut(),
                            PaneNode::new_pane(1, 1),
                        );

                    *self = replacement;
                    return true;
                }

                let second_is_target =
                    matches!(
                        second.as_ref(),
                        PaneNode::Pane(pane)
                            if pane.id() == pane_id
                    );

                if second_is_target {
                    let replacement = 
                        std::mem::replace(
                            first.as_mut(),
                            PaneNode::new_pane(1,1),
                        );
                    
                    *self = replacement;
                    return true;
                }

                if first.remove_pane(pane_id) {
                    return true;
                }

                second.remove_pane(pane_id)
            }
        }
    }

    pub fn for_each_separator<F>(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        callback: &mut F,
    )
    where 
     F: FnMut(
        u32,
        u32,
        u32,
        u32,
     ),
    {
        match self {
            PaneNode::Pane(_) => {}

            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                match direction {
                    SplitDirection::Vertical => {
                        let first_width = (width as f32 * *ratio) as u32;

                        callback(
                            x + first_width,
                            y,
                            1,
                            height,
                        );

                        first.for_each_separator(
                            x,
                            y,
                            first_width,
                            height,
                            callback,
                        );

                        second.for_each_separator(
                            x + first_width,
                            y,
                            width.saturating_sub(first_width),
                            height,
                            callback,
                        );
                    }

                    SplitDirection::Horizontal => {
                        let first_height = (height as f32 * *ratio) as u32;
                        callback(
                            x, 
                            y + first_height,
                            width,
                            1,
                        );

                        first.for_each_separator(
                            x,
                            y,
                            width,
                            first_height,
                            callback,
                        );

                        second.for_each_separator(
                            x,
                            y+ first_height,
                            width,
                            height.saturating_sub(first_height),
                            callback,
                        )
                    }
                }
            }
        }
    }

    pub fn collect_pane_rects(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        rects: &mut Vec<PaneRect>,
    ) {
        match self {
            PaneNode::Pane(pane) => {
                rects.push(
                    PaneRect {
                        pane_id: pane.id(),
                        x,
                        y,
                        width,
                        height,
                    }
                );
            }

            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                match direction {
                    SplitDirection::Vertical => {
                        let first_width = (width as f32 * *ratio) as u32;
                        let second_width = width.saturating_sub(first_width);

                        first.collect_pane_rects(
                            x,
                            y,
                            first_width,
                            height,
                            rects,
                        );

                        second.collect_pane_rects(
                            x + first_width,
                            y,
                            second_width,
                            height,
                            rects,
                        );
                    }

                    SplitDirection::Horizontal => {
                        let first_height = (height as f32 * *ratio) as u32;
                        let second_height = height.saturating_sub(first_height);

                        first.collect_pane_rects(
                            x,
                            y,
                            width,
                            first_height,
                            rects,
                        );

                        second.collect_pane_rects(
                            x,
                            y + first_height,
                            width,
                            second_height,
                            rects,
                        );
                    }
                }
            }
        }
    }
}