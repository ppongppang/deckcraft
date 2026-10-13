//! IME preedit is a UI overlay; only a committed string enters the document and undo history.

#[derive(Default, Debug)]
pub(crate) struct Composition {
    pub(crate) preedit: String,
    cancelled: bool,
}

impl Composition {
    pub(crate) fn is_composing(&self) -> bool {
        !self.preedit.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.preedit.clear();
    }

    pub(crate) fn cancel(&mut self) {
        self.cancelled |= self.is_composing();
        self.clear();
    }

    pub(crate) fn event(&mut self, event: egui::ImeEvent) -> Option<String> {
        match event {
            egui::ImeEvent::Preedit { text, .. } => {
                if !text.is_empty() {
                    self.cancelled = false;
                }
                self.preedit = text;
                None
            }
            egui::ImeEvent::Commit(text) => {
                self.clear();
                let cancelled = std::mem::take(&mut self.cancelled);
                (!cancelled && !text.is_empty()).then_some(text)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn korean_syllable_updates_do_not_insert_intermediate_jamo() {
        let mut composition = Composition::default();
        for text in ["ㄱ", "가", "간"] {
            assert!(composition.event(egui::ImeEvent::Preedit { text: text.into(), active_range_chars: None }).is_none());
            assert_eq!(composition.preedit, text);
        }
        assert_eq!(composition.event(egui::ImeEvent::Commit("간".into())), Some("간".into()));
        assert!(!composition.is_composing());
    }

    #[test]
    fn cancelled_preedit_never_produces_document_text() {
        let mut composition = Composition::default();
        composition.event(egui::ImeEvent::Preedit { text: "한".into(), active_range_chars: None });
        assert!(composition.event(egui::ImeEvent::Preedit { text: String::new(), active_range_chars: None }).is_none());
        assert!(!composition.is_composing());
        assert!(composition.event(egui::ImeEvent::Commit(String::new())).is_none());
    }

    #[test]
    fn switching_away_drops_a_late_commit_from_the_old_target() {
        let mut composition = Composition::default();
        composition.event(egui::ImeEvent::Preedit { text: "한".into(), active_range_chars: None });
        composition.cancel();
        assert!(composition.event(egui::ImeEvent::Commit("한".into())).is_none());
        composition.event(egui::ImeEvent::Preedit { text: "글".into(), active_range_chars: None });
        assert_eq!(composition.event(egui::ImeEvent::Commit("글".into())), Some("글".into()));
    }
}
