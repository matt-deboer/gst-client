//! Defines [`GstClient`] for communication with
//! [`GStreamer Daemon`][1] API.
//!
//! [1]: https://developer.ridgerun.com/wiki/index.php/GStreamer_Daemon
use crate::{gstd_types, resources, Error};
use reqwest::{Client, Response};
use url::Url;

/// [`GstClient`] for [`GStreamer Daemon`][1] API.
///
/// [1]: https://developer.ridgerun.com/wiki/index.php/GStreamer_Daemon
#[derive(Debug, Clone)]
pub struct GstClient {
    http_client: Client,
    pub(crate) base_url: Url,
}

impl GstClient {
    /// Build [`GstClient`] for future call to [`GStreamer Daemon`][1] API.
    ///
    /// # Errors
    ///
    /// If incorrect `base_url` passed
    ///
    /// [1]: https://developer.ridgerun.com/wiki/index.php/GStreamer_Daemon
    pub fn build<S: Into<String>>(base_url: S) -> Result<Self, Error> {
        Ok(Self {
            http_client: Client::new(),
            base_url: Url::parse(&base_url.into()).map_err(Error::IncorrectBaseUrl)?,
        })
    }

    pub(crate) async fn get(&self, url: &str) -> Result<Response, Error> {
        self.http_client
            .get(self.base_url.join(url).map_err(Error::IncorrectApiUrl)?)
            .send()
            .await
            .map_err(Error::RequestFailed)
    }

    pub(crate) async fn post(&self, url: &str) -> Result<Response, Error> {
        self.http_client
            .post(self.base_url.join(url).map_err(Error::IncorrectApiUrl)?)
            .send()
            .await
            .map_err(Error::RequestFailed)
    }

    pub(crate) async fn put(&self, url: &str) -> Result<Response, Error> {
        self.http_client
            .put(self.base_url.join(url).map_err(Error::IncorrectApiUrl)?)
            .send()
            .await
            .map_err(Error::RequestFailed)
    }

    pub(crate) async fn delete(&self, url: &str) -> Result<Response, Error> {
        self.http_client
            .delete(self.base_url.join(url).map_err(Error::IncorrectApiUrl)?)
            .send()
            .await
            .map_err(Error::RequestFailed)
    }

    pub(crate) async fn process_resp(&self, resp: Response) -> Result<gstd_types::Response, Error> {
        if !resp.status().is_success() {
            return Err(Error::BadStatus(resp.status()));
        }

        let res = resp
            .json::<gstd_types::Response>()
            .await
            .map_err(Error::BadBody)?;

        if res.code != gstd_types::ResponseCode::Success {
            return Err(Error::GstdError(res.code));
        }
        Ok(res)
    }

    /// Performs `GET /pipelines` API request, returning the
    /// parsed [`gstd_types::Response`]
    ///
    /// # Errors
    ///
    /// If API request cannot be performed, or fails.
    /// See [`Error`] for details.
    pub async fn pipelines(&self) -> Result<gstd_types::Response, Error> {
        let resp = self.get("pipelines").await?;
        self.process_resp(resp).await
    }
    /// Operate with [`GStreamer Daemon`][1] pipelines.
    ///
    /// # Arguments
    ///
    /// * `name` - name of the pipeline
    ///
    /// [1]: https://developer.ridgerun.com/wiki/index.php/GStreamer_Daemon
    #[must_use]
    pub fn pipeline<S>(&self, name: S) -> resources::Pipeline
    where
        S: Into<String>,
    {
        resources::Pipeline::new(name, self)
    }
    /// Manage [`GStreamer Daemon`][1] Debug mode.
    ///
    /// [1]: https://developer.ridgerun.com/wiki/index.php/GStreamer_Daemon
    #[must_use]
    pub fn debug(&self) -> resources::Debug {
        resources::Debug::new(self)
    }
}

impl Default for GstClient {
    fn default() -> Self {
        Self {
            http_client: Client::new(),
            base_url: Url::parse("http://127.0.0.1:5000").unwrap(),
        }
    }
}

impl From<Url> for GstClient {
    fn from(url: Url) -> Self {
        Self {
            http_client: Client::new(),
            base_url: url,
        }
    }
}

impl From<&Url> for GstClient {
    fn from(url: &Url) -> Self {
        Self {
            http_client: Client::new(),
            base_url: url.clone(),
        }
    }
}

#[cfg(test)]
mod spec {
    use super::*;
    use mockito::{self, mock, Matcher};
    // const BASE_URL: &'static str = "http://10.211.55.4:5000";
    const PIPELINE_NAME: &str = "test_pipeline";
    const PIPELINE_DESC: &str = "videotestsrc pattern=ball";
    const PROJECT_ROOT: &str = env!("CARGO_MANIFEST_DIR");

    fn expect_url() -> Url {
        Url::parse(mockito::server_url().as_str()).unwrap()
    }

    #[test]
    fn create_client_with_build() {
        let client = GstClient::build(mockito::server_url().as_str()).unwrap();
        assert_eq!(client.base_url, expect_url());

        let client = GstClient::build(mockito::server_url()).unwrap();
        assert_eq!(client.base_url, expect_url());
    }

    #[test]
    fn create_client_from() {
        let url = expect_url();
        let client = GstClient::from(&url);
        assert_eq!(client.base_url, expect_url());

        let client = GstClient::from(url);
        assert_eq!(client.base_url, expect_url());
    }

    #[tokio::test]
    async fn create_pipeline() {
        let _m = mock("POST", "/pipelines")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("name".into(), PIPELINE_NAME.into()),
                Matcher::UrlEncoded("description".into(), PIPELINE_DESC.into()),
            ]))
            .with_body_from_file(format!("{PROJECT_ROOT}/tests/files/create_pipeline.json"))
            .create();

        if let Ok(client) = GstClient::build(mockito::server_url().as_str()) {
            let res = client.pipeline(PIPELINE_NAME).create(PIPELINE_DESC).await;
            println!("{:?}", res);
            assert!(res.is_ok());
        };
    }

    #[tokio::test]
    async fn retrieve_pipelines() {
        let _m = mock("GET", "/pipelines")
            .with_body_from_file(format!(
                "{PROJECT_ROOT}/tests/files/retrieve_pipelines.json"
            ))
            .create();

        if let Ok(client) = GstClient::build(mockito::server_url().as_str()) {
            let res = client.pipelines().await;
            println!("{:?}", res);
            assert!(res.is_ok());

            match res {
                Err(e) => {
                    panic!("Unexpected error: {e}");
                }
                Ok(r) => match r.response {
                    gstd_types::ResponseT::Properties(props) => {
                        assert_eq!(props.nodes.len(), 1);
                        assert_eq!(props.nodes[0].name.as_str(), PIPELINE_NAME);
                    }
                    _ => {
                        panic!("Unexpected response type");
                    }
                },
            }
        };
    }

    #[tokio::test]
    async fn retrieve_pipelines_empty() {
        let _m = mock("GET", "/pipelines")
            .with_body_from_file(format!(
                "{PROJECT_ROOT}/tests/files/retrieve_pipelines_empty.json"
            ))
            .create();

        if let Ok(client) = GstClient::build(mockito::server_url().as_str()) {
            let res = client.pipelines().await;
            println!("{:?}", res);
            assert!(res.is_ok());

            match res {
                Err(e) => {
                    panic!("Unexpected error: {e}");
                }
                Ok(r) => match r.response {
                    gstd_types::ResponseT::Properties(props) => {
                        assert!(props.nodes.is_empty());
                    }
                    _ => {
                        panic!("Unexpected response type");
                    }
                },
            }
        };
    }

    #[tokio::test]
    async fn retrieve_pipeline_graph() {
        let _m = mock("GET", format!("/pipelines/{PIPELINE_NAME}/graph").as_str())
            .with_body_from_file(format!(
                "{PROJECT_ROOT}/tests/files/retrieve_pipeline_graph.json"
            ))
            .create();

        if let Ok(client) = GstClient::build(mockito::server_url().as_str()) {
            let res = client.pipeline(PIPELINE_NAME).graph().await;
            println!("{:?}", res);
            assert!(res.is_ok());
        };
    }

    #[tokio::test]
    async fn retrieve_pipeline_elements() {
        let _m = mock(
            "GET",
            format!("/pipelines/{PIPELINE_NAME}/elements").as_str(),
        )
        .with_body_from_file(format!(
            "{PROJECT_ROOT}/tests/files/retrieve_pipeline_elements.json"
        ))
        .create();

        if let Ok(client) = GstClient::build(mockito::server_url().as_str()) {
            let res = client.pipeline(PIPELINE_NAME).elements().await;
            println!("{:?}", res);
            assert!(res.is_ok());
        };
    }
    #[tokio::test]
    async fn retrieve_pipeline_properties() {
        let _m = mock("GET", format!("/pipelines/{PIPELINE_NAME}").as_str())
            .with_body_from_file(format!(
                "{PROJECT_ROOT}/tests/files/retrieve_pipeline_properties.json"
            ))
            .create();

        if let Ok(client) = GstClient::build(mockito::server_url().as_str()) {
            let res = client.pipeline(PIPELINE_NAME).properties().await;
            println!("{:?}", res);
            assert!(res.is_ok());
        };
    }
    #[tokio::test]
    async fn retrieve_pipeline_element_property() {
        let _m = mock(
            "GET",
            format!("/pipelines/{PIPELINE_NAME}/elements/videotestsrc0/properties/is-live")
                .as_str(),
        )
        .with_body_from_file(format!(
            "{PROJECT_ROOT}/tests/files/retrieve_element_property.json"
        ))
        .create();

        if let Ok(client) = GstClient::build(mockito::server_url().as_str()) {
            let res = client
                .pipeline(PIPELINE_NAME)
                .element("videotestsrc0")
                .property("is-live")
                .await;
            println!("{:?}", res);
            assert!(res.is_ok());
        };
    }
    // #[tokio::test]
    // async fn retrieve_pipeline_bus_read() {
    //     let _m = mock("GET", format!("/pipelines/{PIPELINE_NAME}").as_str())
    //             .with_body_from_file(format!("{PROJECT_ROOT}/tests/files/retrieve_element_property.json"))
    //             .create();

    //     if let Ok(client) = GstClient::build(mockito::server_url().as_str()) {
    //         let res = client.pipeline(PIPELINE_NAME).bus().read().await;
    //         println!("{:?}", res);
    //         assert!(res.is_ok());
    //     };
    // }
}
