/// Identity of a shortcut capture and its dialog. Service responses belong to a
/// request; button events additionally belong to the dialog that emitted them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RequestMetadata<Id> {
    request_id: Option<Id>,
    dialog_id: Option<Id>,
}

impl<Id> Default for RequestMetadata<Id> {
    fn default() -> Self {
        Self {
            request_id: None,
            dialog_id: None,
        }
    }
}

impl<Id: Copy + Eq> RequestMetadata<Id> {
    pub(crate) fn new(request_id: Id) -> Self {
        Self {
            request_id: Some(request_id),
            dialog_id: None,
        }
    }

    pub(crate) fn request_id(&self) -> Option<Id> {
        self.request_id
    }

    pub(crate) fn dialog_id(&self) -> Option<Id> {
        self.dialog_id
    }

    pub(crate) fn matches_request(&self, request_id: Id) -> bool {
        self.request_id == Some(request_id)
    }

    pub(crate) fn matches_dialog(&self, request_id: Id, dialog_id: Id) -> bool {
        self.matches_request(request_id) && self.dialog_id == Some(dialog_id)
    }

    pub(crate) fn bind_dialog(&mut self, request_id: Id, dialog_id: Id) -> bool {
        if !self.matches_request(request_id) || self.dialog_id.is_some() {
            return false;
        }
        self.dialog_id = Some(dialog_id);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::RequestMetadata;

    #[test]
    fn response_target_can_exist_before_initial_service_response() {
        let mut request = RequestMetadata::new(1);
        assert!(request.matches_request(1));
        assert_eq!(request.dialog_id(), None);
        assert!(request.bind_dialog(1, 10));
        assert!(request.matches_dialog(1, 10));
    }

    #[test]
    fn old_service_keys_and_null_do_not_match_replacement() {
        let request = RequestMetadata::new(2);
        // Both Some(keys) and None use this check before touching the capture.
        assert!(!request.matches_request(1));
        assert!(request.matches_request(2));
    }

    #[test]
    fn stale_start_failure_cannot_reset_new_request() {
        let mut request = RequestMetadata::new(2);
        if request.matches_request(1) {
            request = RequestMetadata::default();
        }
        assert_eq!(request.request_id(), Some(2));
    }

    #[test]
    fn accept_and_cancel_require_both_request_and_dialog_identity() {
        let mut request = RequestMetadata::new(2);
        assert!(request.bind_dialog(2, 20));
        assert!(!request.matches_dialog(1, 10));
        assert!(!request.matches_dialog(1, 20));
        assert!(!request.matches_dialog(2, 10));
        assert!(request.matches_dialog(2, 20));
    }

    #[test]
    fn dialog_is_not_rebound_by_delayed_service_response() {
        let mut request = RequestMetadata::new(2);
        assert!(!request.bind_dialog(1, 10));
        assert!(request.bind_dialog(2, 20));
        assert!(!request.bind_dialog(2, 21));
        assert_eq!(request.dialog_id(), Some(20));
    }

    #[test]
    fn terminal_reset_rejects_repeated_buttons_and_service_tail() {
        let mut request = RequestMetadata::new(2);
        assert!(request.bind_dialog(2, 20));
        if request.matches_dialog(2, 20) {
            request = RequestMetadata::default();
        }
        assert!(!request.matches_request(2));
        assert!(!request.matches_dialog(2, 20));
        assert!(!request.bind_dialog(2, 21));
    }
}
