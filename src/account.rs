use crate::client::*;
use crate::errors::*;
use crate::rest_model::*;
use crate::util::*;
use serde_json::from_str;
use std::collections::BTreeMap;

static API_V3_ACCOUNT: &str = "/api/v3/account";
static API_V3_OPEN_ORDERS: &str = "/api/v3/openOrders";
static API_V3_ALL_ORDERS: &str = "/api/v3/allOrders";
static API_V3_MYTRADES: &str = "/api/v3/myTrades";
static API_V3_ORDER: &str = "/api/v3/order";
static API_VIRTUAL_SUB_ACCOUNT: &str = "/sapi/v1/sub-account/virtualSubAccount";
/// Endpoint for test orders.
/// Orders issued to this endpoint are validated, but not sent into the matching engine.
static API_V3_ORDER_TEST: &str = "/api/v3/order/test";

/// Account API access, full example provided in examples/binance_endpoints.rs
#[derive(Clone)]
pub struct Account {
    pub client: Client,
    pub recv_window: u64,
}

/// Order Request
/// perform an order for the account
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderRequest {
    pub symbol: String,
    pub side: OrderSide,
    #[serde(rename = "type")]
    pub order_type: OrderType,
    pub time_in_force: Option<TimeInForce>,
    pub quantity: Option<f64>,
    pub quote_order_qty: Option<f64>,
    pub price: Option<f64>,
    /// A unique id for the order, automatically generated if not sent.
    pub new_client_order_id: Option<String>,
    /// Used with stop loss, stop loss limit, take profit and take profit limit order types.
    pub stop_price: Option<f64>,
    /// Used with limit, stop loss limit and take profit limit to create an iceberg order.
    pub iceberg_qty: Option<f64>,
    /// Set the response json, market and limit default to full others to ack.
    pub new_order_resp_type: Option<OrderResponse>,
    /// Cannot be greater than 60000
    pub recv_window: Option<u64>,
}

impl OrderRequest {
    fn valid(&self) -> Result<()> {
        if self.iceberg_qty.is_some() && self.time_in_force != Some(TimeInForce::GTC) {
            return Err(Error::InvalidOrderError {
                msg: "Time in force has to be GTC for iceberg orders".to_string(),
            });
        }
        Ok(())
    }
}

/// Order Cancellation Request
/// perform an order cancellation for the account
/// only works if the parameters match an active order
/// either order_id (binance side id) or orig_client_order_id (id originally given by the client) must be set
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderCancellation {
    pub symbol: String,
    pub order_id: Option<u64>,
    pub orig_client_order_id: Option<String>,
    /// Used to uniquely identify this cancel. Automatically generated by default.
    pub new_client_order_id: Option<String>,
    /// Cannot be greater than 60000
    pub recv_window: Option<u64>,
}

/// Order Status Request
/// perform an order status request for the account
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderStatusRequest {
    pub symbol: String,
    pub order_id: Option<u64>,
    pub orig_client_order_id: Option<String>,
    /// Cannot be greater than 60000
    pub recv_window: Option<u64>,
}

/// Order Status Request
/// perform a query on all orders for the account
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrdersQuery {
    pub symbol: String,
    pub order_id: Option<u64>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    /// Default 500 max 1000
    pub limit: Option<u32>,
    /// Cannot be greater than 60000
    pub recv_window: Option<u64>,
}

impl Account {
    /// General account information
    /// # Examples
    /// ```rust,no_run
    /// use binance::{api::*, account::*, config::*};
    /// let account: Account = Binance::new_with_env(&Config::testnet());
    /// let account = tokio_test::block_on(account.get_account());
    /// assert!(account.is_ok(), "{:?}", account);
    /// ```
    pub async fn get_account(&self) -> Result<AccountInformation> {
        let parameters: BTreeMap<String, String> = BTreeMap::new();

        let request = build_signed_request(parameters, self.recv_window)?;
        let data = self.client.get_signed(API_V3_ACCOUNT, &request).await?;
        let account_info: AccountInformation = from_str(data.as_str())?;

        Ok(account_info)
    }

    /// Account balance for a single asset
    /// # Examples
    /// ```rust,no_run
    /// use binance::{api::*, account::*, config::*};
    /// let account: Account = Binance::new_with_env(&Config::testnet());
    /// let balance = tokio_test::block_on(account.get_balance("BTC"));
    /// assert!(balance.is_ok(), "{:?}", balance);
    /// ```
    pub async fn get_balance<S>(&self, asset: S) -> Result<Balance>
    where
        S: Into<String>,
    {
        match self.get_account().await {
            Ok(account) => {
                let cmp_asset = asset.into();
                for balance in account.balances {
                    if balance.asset == cmp_asset {
                        return Ok(balance);
                    }
                }
                Err(Error::Msg("Asset not found".to_string()))
            }
            Err(e) => Err(e),
        }
    }

    /// All currently open orders for a single symbol
    /// # Examples
    /// ```rust,no_run
    /// use binance::{api::*, account::*, config::*};
    /// let account: Account = Binance::new_with_env(&Config::testnet());
    /// let orders = tokio_test::block_on(account.get_open_orders("BTCUSDT"));
    /// assert!(orders.is_ok(), "{:?}", orders);
    /// ```
    pub async fn get_open_orders<S>(&self, symbol: S) -> Result<Vec<Order>>
    where
        S: Into<String>,
    {
        let mut parameters: BTreeMap<String, String> = BTreeMap::new();
        parameters.insert("symbol".into(), symbol.into());

        let request = build_signed_request(parameters, self.recv_window)?;
        let data = self.client.get_signed(API_V3_OPEN_ORDERS, &request).await?;
        let order: Vec<Order> = from_str(data.as_str())?;

        Ok(order)
    }

    /// All orders for the account
    /// # Examples
    /// ```rust,no_run
    /// use binance::{api::*, account::*, config::*};
    /// let account: Account = Binance::new_with_env(&Config::testnet());
    /// let query = OrdersQuery {
    ///     symbol: "BTCUSDT".to_string(),
    ///     order_id: None,
    ///     start_time: None,
    ///     end_time: None,
    ///     limit: None,
    ///     recv_window: None,
    /// };
    /// let orders = tokio_test::block_on(account.get_all_orders(query));
    /// assert!(orders.is_ok(), "{:?}", orders);
    /// ```
    pub async fn get_all_orders(&self, query: OrdersQuery) -> Result<Vec<Order>> {
        let recv_window = query.recv_window.unwrap_or(self.recv_window);
        let request = build_signed_request_p(query, recv_window)?;
        let data = self.client.get_signed(API_V3_ALL_ORDERS, &request).await?;
        let order: Vec<Order> = from_str(data.as_str())?;

        Ok(order)
    }

    /// All currently open orders for the account
    /// # Examples
    /// ```rust,no_run
    /// use binance::{api::*, account::*, config::*};
    /// let account: Account = Binance::new_with_env(&Config::testnet());
    /// let orders = tokio_test::block_on(account.get_all_open_orders());
    /// assert!(orders.is_ok(), "{:?}", orders);
    /// ```
    pub async fn get_all_open_orders(&self) -> Result<Vec<Order>> {
        let request = build_signed_request(BTreeMap::new(), self.recv_window)?;
        let data = self.client.get_signed(API_V3_OPEN_ORDERS, &request).await?;
        let order: Vec<Order> = from_str(data.as_str())?;

        Ok(order)
    }

    /// Cancels all currently open orders of specified symbol for the account
    /// # Examples
    /// ```rust,no_run
    /// use binance::{api::*, account::*, config::*};
    /// let account: Account = Binance::new_with_env(&Config::testnet());
    /// let canceled_orders = tokio_test::block_on(account.cancel_all_open_orders());
    /// assert!(canceled_orders.is_ok(), "{:?}", canceled_orders);
    /// ```
    pub async fn cancel_all_open_orders<S>(&self, symbol: S) -> Result<Vec<Order>>
    where
        S: Into<String>,
    {
        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("symbol".into(), symbol.into());
        let request = build_signed_request(params, self.recv_window)?;
        let data = self.client.delete_signed(API_V3_OPEN_ORDERS, &request).await?;
        let order: Vec<Order> = from_str(data.as_str())?;
        Ok(order)
    }

    /// Check an order's status
    /// # Examples
    /// ```rust,no_run
    /// use binance::{api::*, account::*, config::*};
    /// let account: Account = Binance::new_with_env(&Config::testnet());
    /// let query = OrderStatusRequest {
    ///     symbol: "BTCUSDT".to_string(),
    ///     order_id: Some(1),
    ///     orig_client_order_id: Some("my_id".to_string()),
    ///     recv_window: None
    /// };
    /// let order = tokio_test::block_on(account.order_status(query));
    /// assert!(order.is_ok(), "{:?}", order);
    /// ```
    pub async fn order_status(&self, osr: OrderStatusRequest) -> Result<Order> {
        let recv_window = osr.recv_window.unwrap_or(self.recv_window);
        let request = build_signed_request_p(osr, recv_window)?;
        let data = self.client.get_signed(API_V3_ORDER, &request).await?;
        let order: Order = from_str(data.as_str())?;

        Ok(order)
    }

    /// Place a test status order
    ///
    /// This order is sandboxed: it is validated, but not sent to the matching engine.
    /// # Examples
    /// ```rust,no_run
    /// use binance::{api::*, account::*, config::*};
    /// let account: Account = Binance::new_with_env(&Config::testnet());
    /// let query = OrderStatusRequest {
    ///     symbol: "BTCUSDT".to_string(),
    ///     order_id: Some(1),
    ///     orig_client_order_id: Some("my_id".to_string()),
    ///     recv_window: None
    /// };
    /// let resp = tokio_test::block_on(account.test_order_status(query));
    /// assert!(resp.is_ok(), "{:?}", resp);
    /// ```
    pub async fn test_order_status(&self, osr: OrderStatusRequest) -> Result<TestResponse> {
        let recv_window = osr.recv_window.unwrap_or(self.recv_window);
        let request = build_signed_request_p(osr, recv_window)?;
        let data = self.client.get_signed(API_V3_ORDER_TEST, &request).await?;
        let tr: TestResponse = from_str(data.as_str())?;

        Ok(tr)
    }

    /// Place an order
    /// Returns the Transaction if Ok
    /// This methods validates the order request before sending, making sure it complies with Binance rules
    /// # Examples
    /// ```rust,no_run
    /// use binance::{api::*, account::*, config::*, rest_model::*};
    /// let account: Account = Binance::new_with_env(&Config::testnet());
    /// let limit_buy = OrderRequest {
    ///         symbol: "BTCUSDT".to_string(),
    ///         quantity: Some(10.0),
    ///         price: Some(0.014000),
    ///         order_type: OrderType::Limit,
    ///         side: OrderSide::Buy,
    ///         time_in_force: Some(TimeInForce::FOK),
    ///         ..OrderRequest::default()
    ///     };
    /// let transaction = tokio_test::block_on(account.place_order(limit_buy));
    /// assert!(transaction.is_ok(), "{:?}", transaction);
    /// ```
    pub async fn place_order(&self, order: OrderRequest) -> Result<Transaction> {
        let _ = order.valid()?;
        let recv_window = order.recv_window.unwrap_or(self.recv_window);
        let request = build_signed_request_p(order, recv_window)?;
        let data = self.client.post_signed(API_V3_ORDER, &request).await?;
        let transaction: Transaction = from_str(data.as_str())?;

        Ok(transaction)
    }

    /// Place a test order
    ///
    /// Despite being a test, this order is still validated before calls
    /// This order is sandboxed: it is validated, but not sent to the matching engine.
    /// # Examples
    /// ```rust,no_run
    /// use binance::{api::*, account::*, config::*, rest_model::*};
    /// let account: Account = Binance::new_with_env(&Config::testnet());
    /// let limit_buy = OrderRequest {
    ///         symbol: "BTCUSDT".to_string(),
    ///         quantity: Some(10.0),
    ///         price: Some(0.014000),
    ///         order_type: OrderType::Limit,
    ///         side: OrderSide::Buy,
    ///         time_in_force: Some(TimeInForce::FOK),
    ///         ..OrderRequest::default()
    ///     };
    /// let resp = tokio_test::block_on(account.place_test_order(limit_buy));
    /// assert!(resp.is_ok(), "{:?}", resp);
    /// ```
    pub async fn place_test_order(&self, order: OrderRequest) -> Result<TestResponse> {
        let _ = order.valid()?;
        let recv_window = order.recv_window.unwrap_or(self.recv_window);
        let request = build_signed_request_p(order, recv_window)?;
        let data = self.client.post_signed(API_V3_ORDER_TEST, &request).await?;
        let tr: TestResponse = from_str(data.as_str())?;
        Ok(tr)
    }

    /// Place a cancellation order
    /// # Examples
    /// ```rust,no_run
    /// use binance::{api::*, account::*, config::*};
    /// let account: Account = Binance::new_with_env(&Config::testnet());
    /// let query = OrderCancellation {
    ///     symbol: "BTCUSDT".to_string(),
    ///     order_id: Some(1),
    ///     orig_client_order_id: Some("my_id".to_string()),
    ///     new_client_order_id: None,
    ///     recv_window: None
    /// };
    /// let canceled = tokio_test::block_on(account.cancel_order(query));
    /// assert!(canceled.is_ok(), "{:?}", canceled);
    /// ```
    pub async fn cancel_order(&self, o: OrderCancellation) -> Result<OrderCanceled> {
        let recv_window = o.recv_window.unwrap_or(self.recv_window);
        let request = build_signed_request_p(o, recv_window)?;
        let data = self.client.delete_signed(API_V3_ORDER, &request).await?;
        let order_canceled: OrderCanceled = from_str(data.as_str())?;

        Ok(order_canceled)
    }

    /// Place a test cancel order
    ///
    /// This order is sandboxed: it is validated, but not sent to the matching engine.
    /// # Examples
    /// ```rust,no_run
    /// use binance::{api::*, account::*, config::*};
    /// let account: Account = Binance::new_with_env(&Config::testnet());
    /// let query = OrderCancellation {
    ///     symbol: "BTCUSDT".to_string(),
    ///     order_id: Some(1),
    ///     orig_client_order_id: Some("my_id".to_string()),
    ///     new_client_order_id: None,
    ///     recv_window: None
    /// };
    /// let response = tokio_test::block_on(account.test_cancel_order(query));
    /// assert!(response.is_ok(), "{:?}", response);
    /// ```
    pub async fn test_cancel_order(&self, o: OrderCancellation) -> Result<TestResponse> {
        let recv_window = o.recv_window.unwrap_or(self.recv_window);
        let request = build_signed_request_p(o, recv_window)?;
        let data = self.client.delete_signed(API_V3_ORDER_TEST, &request).await?;
        let tr: TestResponse = from_str(data.as_str())?;

        Ok(tr)
    }

    /// Trade history
    /// # Examples
    /// ```rust,no_run
    /// use binance::{api::*, account::*, config::*};
    /// let account: Account = Binance::new_with_env(&Config::testnet());
    /// let trade_history = tokio_test::block_on(account.trade_history("BTCUSDT"));
    /// assert!(trade_history.is_ok(), "{:?}", trade_history);
    /// ```
    pub async fn trade_history<S>(&self, symbol: S) -> Result<Vec<TradeHistory>>
    where
        S: Into<String>,
    {
        let mut parameters: BTreeMap<String, String> = BTreeMap::new();
        parameters.insert("symbol".into(), symbol.into());

        let request = build_signed_request(parameters, self.recv_window)?;
        let data = self.client.get_signed(API_V3_MYTRADES, &request).await?;
        let trade_history: Vec<TradeHistory> = from_str(data.as_str())?;

        Ok(trade_history)
    }

    pub async fn create_sub_account<S>(&self, label: S) -> Result<SubAccountCreationResp>
    where
        S: Into<String>,
    {
        let request = build_signed_request_p(
            SubAccountCreationReq {
                sub_account_string: label.into(),
            },
            self.recv_window,
        )?;
        let data = self.client.post_signed(API_VIRTUAL_SUB_ACCOUNT, &request).await?;
        let resp: SubAccountCreationResp = from_str(data.as_str())?;
        Ok(resp)
    }
}
