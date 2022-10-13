use anyhow::Result as AnyResult;
use schemars::JsonSchema;
use std::fmt;

use cosmwasm_std::{Addr, Coin, Decimal};
use terra_multi_test::{App, AppBuilder, AppResponse, Contract, ContractWrapper, Executor};

use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, WeightPerProtocol, Whitelist};
use crate::state::Config;

pub fn contract_distributor<C>() -> Box<dyn Contract<C>>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    let contract = ContractWrapper::new_with_empty(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

/// Builder for test suite
#[derive(Debug)]
pub struct SuiteBuilder {
    pub admin: String,
    pub burn_address: String,
    pub developer_address: String,
    pub whitelist: Vec<Whitelist>,
    pub weight_per_protocol: Vec<WeightPerProtocol>,
    pub funds: Vec<(Addr, Vec<Coin>)>,
}

impl SuiteBuilder {
    pub fn new() -> Self {
        Self {
            admin: "owner".to_owned(),
            burn_address: "burnaddress".to_owned(),
            developer_address: "devaddress".to_owned(),
            whitelist: vec![],
            weight_per_protocol: vec![],
            funds: vec![],
        }
    }

    pub fn with_admin(mut self, admin: String) -> Self {
        self.admin = admin;
        self
    }

    pub fn with_whitelist(mut self, whitelist: &[(&str, &str)]) -> Self {
        let mut list = vec![];
        for (address, protocol) in whitelist {
            let entry = Whitelist {
                address: address.to_string(),
                protocol: protocol.to_string(),
            };
            list.push(entry);
        }
        self.whitelist = list;
        self
    }

    pub fn with_weights_per_protocol(mut self, list: &[(&str, u64)]) -> Self {
        let mut weights = vec![];
        for (protocol, weight) in list {
            let entry = WeightPerProtocol {
                protocol: protocol.to_string(),
                weight: Decimal::percent(*weight),
            };
            weights.push(entry);
        }
        self.weight_per_protocol = weights;
        self
    }

    /// Sets initial amount of distributable tokens on address
    pub fn with_funds(mut self, addr: &str, funds: &[Coin]) -> Self {
        self.funds.push((Addr::unchecked(addr), funds.into()));
        self
    }

    #[track_caller]
    pub fn build(self) -> Suite {
        let mut app: App = AppBuilder::new().build();

        let owner = Addr::unchecked("owner");

        let admin = self.admin;
        let burn_address = self.burn_address;
        let developer_address = self.developer_address;

        let distributor_id = app.store_code(contract_distributor());
        let distributor_contract = app
            .instantiate_contract(
                distributor_id,
                owner.clone(),
                &InstantiateMsg {
                    admin,
                    burn_address: burn_address.clone(),
                    developer_address: developer_address.clone(),
                    whitelist: self.whitelist,
                    weight_per_protocol: self.weight_per_protocol,
                },
                &[],
                "distributor",
                None,
            )
            .unwrap();

        let funds = self.funds;
        for (addr, coin) in funds {
            app.init_bank_balance(&addr, coin).unwrap();
        }

        Suite {
            app,
            owner,
            contract: distributor_contract,
            burn_address,
            developer_address,
        }
    }
}

/// Test suite
pub struct Suite {
    /// The multitest app
    app: App,
    owner: Addr,
    /// Address of Market contract
    contract: Addr,
    /// Address of burn contract
    burn_address: String,
    /// Address of developer contract
    developer_address: String,
}

impl Suite {
    pub fn owner(&mut self) -> Addr {
        self.owner.clone()
    }

    pub fn contract(&mut self) -> String {
        self.contract.to_string()
    }

    pub fn burn_address(&mut self) -> String {
        self.burn_address.clone()
    }

    pub fn developer_address(&mut self) -> String {
        self.developer_address.clone()
    }

    pub fn deposit(&mut self, sender: &str, funds: &[Coin]) -> AnyResult<AppResponse> {
        self.app
            .send_tokens(Addr::unchecked(sender), self.contract.clone(), funds)
    }

    pub fn distribute(&mut self, sender: &str, denom: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.contract.clone(),
            &ExecuteMsg::Distribute {
                denom: denom.into(),
            },
            &[],
        )
    }

    pub fn update_config(
        &mut self,
        sender: &str,
        admin: impl Into<Option<String>>,
        burn_address: impl Into<Option<String>>,
        developer_address: impl Into<Option<String>>,
        whitelist: impl Into<Option<Vec<Whitelist>>>,
        weight_per_protocol: impl Into<Option<Vec<WeightPerProtocol>>>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            Addr::unchecked(sender),
            self.contract.clone(),
            &ExecuteMsg::UpdateConfig {
                admin: admin.into(),
                burn_address: burn_address.into(),
                developer_address: developer_address.into(),
                whitelist: whitelist.into(),
                weight_per_protocol: weight_per_protocol.into(),
            },
            &[],
        )
    }

    pub fn query_config(&self) -> AnyResult<Config> {
        let response: Config = self
            .app
            .wrap()
            .query_wasm_smart(self.contract.clone(), &QueryMsg::Config {})?;
        Ok(response)
    }
}
