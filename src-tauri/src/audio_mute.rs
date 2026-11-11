#[derive(Default)]
pub(crate) struct FocusMute {
    saved: Option<bool>,
}

impl FocusMute {
    pub(crate) fn sync<E>(
        &mut self,
        focused: bool,
        current: bool,
        set_muted: impl FnOnce(bool) -> Result<(), E>,
    ) -> Result<(), E> {
        match (focused, self.saved) {
            (false, None) => {
                set_muted(true)?;
                self.saved = Some(current);
            }
            (true, Some(previous)) => {
                set_muted(previous)?;
                self.saved = None;
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_blur_preserves_the_original_unmuted_state() {
        let mut state = FocusMute::default();
        let mut changes = Vec::new();
        state
            .sync(false, false, |value| {
                changes.push(value);
                Ok::<_, ()>(())
            })
            .unwrap();
        state
            .sync(false, true, |value| {
                changes.push(value);
                Ok::<_, ()>(())
            })
            .unwrap();
        state
            .sync(true, true, |value| {
                changes.push(value);
                Ok::<_, ()>(())
            })
            .unwrap();
        state
            .sync(true, false, |value| {
                changes.push(value);
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(changes, [true, false]);
    }

    #[test]
    fn an_already_muted_webview_stays_muted_on_focus() {
        let mut state = FocusMute::default();
        let mut changes = Vec::new();
        state
            .sync(true, true, |value| {
                changes.push(value);
                Ok::<_, ()>(())
            })
            .unwrap();
        state
            .sync(false, true, |value| {
                changes.push(value);
                Ok::<_, ()>(())
            })
            .unwrap();
        state
            .sync(true, true, |value| {
                changes.push(value);
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(changes, [true, true]);
    }

    #[test]
    fn a_failed_mute_is_retried_without_losing_the_original_state() {
        let mut state = FocusMute::default();
        assert_eq!(
            state.sync(false, false, |_| Err("COM failure")),
            Err("COM failure")
        );
        let mut changes = Vec::new();
        state
            .sync(false, false, |value| {
                changes.push(value);
                Ok::<_, ()>(())
            })
            .unwrap();
        state
            .sync(true, true, |value| {
                changes.push(value);
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(changes, [true, false]);
    }

    #[test]
    fn a_failed_restore_is_retried_on_the_next_focus_sync() {
        let mut state = FocusMute::default();
        state.sync(false, false, |_| Ok::<_, ()>(())).unwrap();
        assert_eq!(
            state.sync(true, true, |_| Err("COM failure")),
            Err("COM failure")
        );
        let mut changes = Vec::new();
        state
            .sync(true, true, |value| {
                changes.push(value);
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(changes, [false]);
    }
}
