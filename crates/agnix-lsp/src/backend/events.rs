use super::*;

impl Backend {
    pub(super) async fn handle_did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        {
            let mut docs = self.documents.write().await;
            docs.insert(uri.clone(), Arc::new(text));
        }
        self.validate_from_content_and_publish(uri, None).await;
    }

    pub(super) async fn handle_did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().next() {
            {
                let mut docs = self.documents.write().await;
                docs.insert(uri.clone(), Arc::new(change.text));
            }
            self.validate_from_content_and_publish(uri, None).await;
        }
    }

    pub(super) async fn handle_did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        self.validate_from_content_and_publish(uri.clone(), None)
            .await;

        // Re-run project-level validation when a relevant file is saved
        if let Ok(path) = uri.to_file_path() {
            if Self::is_project_level_trigger(&path) {
                self.spawn_project_validation();
            }
        }
    }

    pub(super) async fn handle_did_close(&self, params: DidCloseTextDocumentParams) {
        {
            let mut docs = self.documents.write().await;
            docs.remove(&params.text_document.uri);
        }
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }
}
