use std::collections::HashMap;

use quote::{InternalQuoteRequest, QuoteRequest, QuoteResponse};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue, InvalidHeaderValue},
    Client, Response,
};
use serde::de::DeserializeOwned;
use swap::{SwapInstructionsResponse, SwapInstructionsResponseInternal, SwapRequest, SwapResponse};
use thiserror::Error;

pub mod quote;
pub mod route_plan_with_metadata;
pub mod serde_helpers;
pub mod swap;
pub mod transaction_config;

#[derive(Clone)]
pub struct JupiterSwapApiClient {
    pub base_path: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Request failed with status {status}: {body}")]
    RequestFailed {
        status: reqwest::StatusCode,
        body: String,
    },
    #[error("Failed to deserialize response: {0}")]
    DeserializationError(#[from] reqwest::Error),
    #[error("Invalid header: {0}")]
    InvalidHeader(#[from] InvalidHeaderValue),
}

async fn check_is_success(response: Response) -> Result<Response, ClientError> {
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(ClientError::RequestFailed { status, body });
    }
    Ok(response)
}

async fn check_status_code_and_deserialize<T: DeserializeOwned>(
    response: Response,
) -> Result<T, ClientError> {
    let response = check_is_success(response).await?;
    response
        .json::<T>()
        .await
        .map_err(ClientError::DeserializationError)
}

impl JupiterSwapApiClient {
    pub fn new(base_path: String, api_key: Option<String>) -> Self {
        Self { base_path, api_key }
    }

    pub async fn quote(&self, quote_request: &QuoteRequest) -> Result<QuoteResponse, ClientError> {
        let url = format!("{}/quote", self.base_path);
        let extra_args = quote_request.quote_args.clone();
        let internal_quote_request = InternalQuoteRequest::from(quote_request.clone());
        let mut headers = HeaderMap::new();
        if let Some(api_key) = &self.api_key {
            headers.insert(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_str(api_key).map_err(ClientError::InvalidHeader)?,
            );
        }
        let response = Client::new()
            .get(url)
            .query(&internal_quote_request)
            .query(&extra_args)
            .headers(headers)
            .send()
            .await?;
        check_status_code_and_deserialize(response).await
    }

    pub async fn swap(
        &self,
        swap_request: &SwapRequest,
        extra_args: Option<HashMap<String, String>>,
    ) -> Result<SwapResponse, ClientError> {
        let mut headers = HeaderMap::new();
        if let Some(api_key) = &self.api_key {
            headers.insert(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_str(api_key).map_err(ClientError::InvalidHeader)?,
            );
        }
        let response = Client::new()
            .post(format!("{}/swap", self.base_path))
            .query(&extra_args)
            .json(swap_request)
            .headers(headers)
            .send()
            .await?;
        check_status_code_and_deserialize(response).await
    }

    pub async fn swap_instructions(
        &self,
        swap_request: &SwapRequest,
    ) -> Result<SwapInstructionsResponse, ClientError> {
        let mut headers = HeaderMap::new();
        if let Some(api_key) = &self.api_key {
            headers.insert(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_str(api_key).map_err(ClientError::InvalidHeader)?,
            );
        }
        let response = Client::new()
            .post(format!("{}/swap-instructions", self.base_path))
            .json(swap_request)
            .headers(headers)
            .send()
            .await?;
        check_status_code_and_deserialize::<SwapInstructionsResponseInternal>(response)
            .await
            .map(Into::into)
    }
}
