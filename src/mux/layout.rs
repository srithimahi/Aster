use super::pane::Pane;

#[derive(Clone, Copy, Debug)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
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
}