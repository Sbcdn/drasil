use super::super::txtools::utxo_handling::input_selection;
use super::error::TransferError;
use crate::clib;
use crate::clib::{
    address::{Address, RewardAddress},
    utils::Value,
    TransactionInputs, TransactionOutput, TransactionOutputs,
};
use crate::modules::txtools::utxo_handling::combine_wallet_outputs;
use crate::TransactionUnspentOutputs;
use clib::utils::BigNum;
use std::fmt::Debug;

#[derive(Clone)]
pub struct CardanoNativeScript {
    pub script_addr: Address,
    pub script: clib::NativeScript,
    pub version: f32,
    pvks: Vec<String>,
}

impl CardanoNativeScript {
    pub fn new(
        script_addr: &Address,
        script: &clib::NativeScript,
        version: f32,
        pvks: Vec<String>,
    ) -> Self {
        CardanoNativeScript {
            script_addr: script_addr.clone(),
            script: script.clone(),
            version,
            pvks,
        }
    }

    pub fn get_pvks_mut(&'_ mut self) -> &'_ mut Vec<String> {
        &mut self.pvks
    }

    pub fn get_pvks(&'_ self) -> &'_ Vec<String> {
        &self.pvks
    }
}

impl std::fmt::Debug for CardanoNativeScript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CardanoNativeScript")
            .field("script_addr", &self.script_addr)
            .field("script", &self.script)
            .field("version", &self.version)
            .field("pvks", &"*****")
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct TransWallet {
    pub stake_addr: Option<RewardAddress>,
    pay_addr: Address,
    pub utxos: TransactionUnspentOutputs,
    pub script: Option<CardanoNativeScript>,
    pub smtcntr: Option<clib::plutus::PlutusScripts>,
    cid: Option<i64>,
}

impl TransWallet {
    pub fn new(pay_addr: &Address, utxos: &TransactionUnspentOutputs) -> Self {
        TransWallet {
            stake_addr: None,
            pay_addr: pay_addr.clone(),
            utxos: utxos.clone(),
            script: None,
            smtcntr: None,
            cid: None,
        }
    }

    pub fn set_native_script(&mut self, scripts: CardanoNativeScript) {
        self.script = Some(scripts);
    }

    pub fn set_plutus_contract(&mut self, plutus: clib::plutus::PlutusScripts) {
        self.smtcntr = Some(plutus);
    }

    pub fn get_payment_addr(&self) -> Address {
        self.pay_addr.clone()
    }

    pub fn set_cid(&mut self, cid: i64) {
        self.cid = Some(cid);
    }

    pub fn get_cid(&self) -> i64 {
        self.cid.unwrap_or(-1)
    }
}

#[derive(Clone, Debug)]
pub struct TransWallets {
    pub wallets: Vec<TransWallet>,
}
impl Default for TransWallets {
    fn default() -> Self {
        Self::new()
    }
}
impl TransWallets {
    pub fn new() -> Self {
        TransWallets {
            wallets: Vec::<TransWallet>::new(),
        }
    }

    pub fn add_wallet(&mut self, wallet: &TransWallet) {
        self.wallets.push(wallet.clone());
    }

    pub fn set_wallets(&mut self, wallets: &[TransWallet]) {
        self.wallets = wallets.to_vec();
    }

    pub fn get_wallets(&self) -> Vec<TransWallet> {
        self.wallets.clone()
    }

    pub fn get_wallet(&self, pay_addr: &Address) -> Result<TransWallet, TransferError> {
        let wallets: Vec<&TransWallet> = self
            .wallets
            .iter()
            .filter(|n| n.pay_addr == *pay_addr)
            .collect();

        match wallets.len() {
            1 => Ok(wallets[0].clone()),
            0 => Err(TransferError::NoWalletForAddress),
            _ => Err(TransferError::TooManyWalletsForAddress),
        }
    }

    pub fn get_wallet_cid(&self, cid: i64) -> Result<TransWallet, TransferError> {
        let wallets: Vec<&TransWallet> =
            self.wallets.iter().filter(|n| n.get_cid() == cid).collect();

        match wallets.len() {
            1 => Ok(wallets[0].clone()),
            0 => Err(TransferError::NoWalletForCID),
            _ => Err(TransferError::TooManyWalletsForAddress),
        }
    }
}

#[derive(Clone)]
pub(crate) struct TransBuilder {
    fee_addr: Address,
    pub wallets: TransWallets,
    pub transfers: Vec<Transfer>,
    pub tx: Option<(
        TransactionUnspentOutputs,
        TransactionInputs,
        TransactionOutputs,
    )>,
}

impl TransBuilder {
    pub fn new(fee_addr: &Address) -> Self {
        TransBuilder {
            fee_addr: fee_addr.clone(),
            wallets: TransWallets::new(),
            transfers: Vec::<Transfer>::new(),
            tx: None,
        }
    }
    /*
       async fn flatten<T>(handle: JoinHandle<Result<T, &'static str>>) -> Result<T, &'static str> {
           match handle.await {
               Ok(Ok(result)) => Ok(result),
               Ok(Err(err)) => Err(err),
               Err(err) => Err("handling failed"),
           }
       }
    */
    pub fn build(&mut self, fee: BigNum) -> Result<(), TransferError> {
        // Apply fees
        let fee_transfer = self.find_transfer(&self.fee_addr);
        if fee_transfer.is_none() {
            return Err(TransferError::Custom(
                "no transfer for specified fee_addr exists".to_string(),
            ));
        }
        log::debug!("\n\n\nBefore FeeSet: \n{:?}\n", self.transfers);
        let mut fee_transfer = fee_transfer.unwrap();
        fee_transfer.0.source.add_fee(&fee)?;
        self.replace_transfer((&fee_transfer.0, fee_transfer.1))?;
        log::debug!("\n\n\nAfter FeeSet: \n{:?}\n", self.transfers);
        // Balance all transfers
        //let mut handles = Vec::<_>::new();

        self.transfers.iter_mut().for_each(|n| {
            log::debug!("Pay Address: {:?}", n.source.pay_addr);
            log::debug!("Wallets: {:?}", self.wallets);
            n.balance(self.wallets.get_wallet(&n.source.pay_addr).unwrap())
                .unwrap();
        });

        //handles.push();

        //let handler = tokio::spawn(async move { t.balance(wallet).await });
        //handles.push(handler);

        //futures::future::join_all(handles.into_iter()).await;

        // Determine total tx inputs
        log::debug!("\n Determine total tx inputs.....");
        let txins: TransactionInputs =
            self.transfers
                .iter()
                .fold(TransactionInputs::new(), |mut acc, n| {
                    log::debug!("\n\nTransfer: {:?}", n);
                    n.txis.iter().for_each(|m| {
                        for i in 0..m.len() {
                            let txi = m.get(i);
                            acc.add(&txi);
                        }
                    });
                    acc
                });

        let txiuos = self
            .transfers
            .iter()
            .fold(TransactionUnspentOutputs::new(), |mut acc, n| {
                acc.merge(n.source.get_tx_unspent_inputs().unwrap_or_default());
                acc
            });

        let txins_val: Value = self.transfers.iter().fold(Value::zero(), |mut acc, n| {
            n.source.txiuo.iter().for_each(|m| {
                acc = acc.checked_add(&m.calc_total_value().unwrap()).unwrap();
            });
            acc
        });

        // Determine total tx outputs
        let txos: TransactionOutputs =
            self.transfers
                .iter()
                .fold(TransactionOutputs::new(), |mut acc, m| {
                    m.txos.iter().for_each(|n| {
                        for i in 0..n.len() {
                            acc.add(&n.get(i));
                        }
                    });
                    acc
                });
        let txos = combine_wallet_outputs(&txos);
        let mut txo_val = Value::zero();
        for j in 0..txos.len() {
            let v = txos.get(j).amount();
            txo_val = txo_val.checked_add(&v).unwrap();
        }
        // Check tx balance is 0 / ToDo: How to do on miniting?
        log::debug!("\n\nTXIn Val: {:?}", txins_val);
        log::debug!("\n\nTXOut Val: {:?}", txo_val);
        log::debug!("\n\nFee Val: {:?}", fee);
        log::debug!("\n\nTxUnspentInputs: {:?}", txiuos);
        log::debug!("\n\nTxOutputs: {:?}", txos);

        if txins_val
            .checked_sub(&txo_val.checked_add(&Value::new(&fee)).unwrap())
            .unwrap()
            .compare(&Value::zero())
            != Some(0)
        {
            return Err(TransferError::TxNotBalanced);
        }

        // Set TXBO
        self.tx = Some((txiuos, txins, txos));

        Ok(())
    }

    pub fn find_transfer(&self, pay_addr: &Address) -> Option<(Transfer, usize)> {
        let t: Vec<(usize, &Transfer)> = self
            .transfers
            .iter()
            .enumerate()
            .filter(|n| n.1.source.pay_addr == *pay_addr)
            .collect();
        match t.len() {
            1 => Some((t[0].1.clone(), t[0].0)),
            _ => None,
        }
    }

    pub fn replace_transfer(&mut self, t: (&Transfer, usize)) -> Result<(), TransferError> {
        self.transfers.swap_remove(t.1);
        self.transfers.push(t.0.clone());
        Ok(())
    }

    pub fn get_fee_addr(&self) -> Address {
        self.fee_addr.clone()
    }
}

#[derive(Clone, Debug)]
pub(crate) enum TransModificator {
    Add(Value),
    Sub(Value),
}

#[derive(Clone, Debug)]
pub(crate) struct Source {
    pay_addr: Address,
    pay_value: Option<Value>,
    txis: Option<TransactionInputs>,
    txiuo: Option<TransactionUnspentOutputs>,
    redeemer: Option<clib::plutus::Redeemers>,
    modificator: Vec<TransModificator>,
}

impl Source {
    pub fn new(pay_addr: &Address) -> Self {
        Source {
            pay_addr: pay_addr.clone(),
            pay_value: None,
            txis: None,
            txiuo: None,
            redeemer: None,
            modificator: Vec::<TransModificator>::new(),
        }
    }

    pub fn add_fee(&mut self, fee: &BigNum) -> Result<(), TransferError> {
        self.add_addition(&Value::new(fee));
        Ok(())
    }

    pub fn add_subtraction(&mut self, sub: &Value) {
        self.modificator.push(TransModificator::Sub(sub.clone()));
    }

    pub fn add_addition(&mut self, sub: &Value) {
        self.modificator.push(TransModificator::Add(sub.clone()));
    }

    pub fn set_pay_value(&mut self, value: Value) {
        self.pay_value = Some(value);
    }

    pub fn set_redeemer(&mut self, redeemer: clib::plutus::Redeemers) {
        self.redeemer = Some(redeemer);
    }

    pub fn set_txinputs(&mut self, txis: TransactionInputs) {
        self.txis = Some(txis);
    }

    pub fn set_tx_unspent_inputs(&mut self, txiuo: TransactionUnspentOutputs) {
        self.txiuo = Some(txiuo);
    }

    pub fn get_modificator(&self) -> Vec<TransModificator> {
        self.modificator.clone()
    }
    pub fn get_pay_value(&self) -> Option<Value> {
        self.pay_value.clone()
    }

    pub fn get_redeemer(&self) -> Option<clib::plutus::Redeemers> {
        self.redeemer.clone()
    }

    pub fn get_txinputs(&self) -> Option<TransactionInputs> {
        self.txis.clone()
    }

    pub fn get_tx_unspent_inputs(&self) -> Option<TransactionUnspentOutputs> {
        self.txiuo.clone()
    }

    pub fn select_txis(&mut self, wallet: &TransWallet) -> Result<(), TransferError> {
        if self.pay_addr != wallet.pay_addr {
            return Err(TransferError::WrongWalletForAddress);
        }
        let mut pval = if self.pay_value.is_some() {
            self.pay_value.as_ref().unwrap().clone()
        } else {
            return Err(TransferError::SourceNoPaymentValueSet);
        };
        for modificator in &self.modificator {
            match modificator {
                TransModificator::Add(v) => {
                    pval = pval.checked_add(v)?;
                }
                TransModificator::Sub(_v) => {
                    //pval = pval.checked_sub(v)?;
                }
            }
        }
        let inputs = input_selection(None, &mut pval, &wallet.utxos, None, Some(&self.pay_addr))?;
        self.set_txinputs(inputs.0);
        self.set_tx_unspent_inputs(inputs.1);
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TransPData {
    pub datum: Option<clib::plutus::PlutusData>,
    pub datum_hash: Option<clib::crypto::DataHash>,
    pub script_ref: Option<clib::ScriptRef>,
}

#[derive(Clone, Debug)]
pub(crate) struct Sink {
    recv_addr: Address,
    pub value: Value,
    pub txos: Option<TransactionOutputs>,
    pub plutus_data: Option<TransPData>,
}

impl Sink {
    pub fn new(recv_addr: &Address, value: &Value) -> Self {
        Sink {
            recv_addr: recv_addr.clone(),
            value: value.clone(),
            txos: None,
            plutus_data: None,
        }
    }

    fn make_outputs(&mut self) -> Result<(), super::error::TransferError> {
        let mut txo = TransactionOutput::new(&self.get_recv_addr(), &self.value);
        if let Some(data) = &self.plutus_data {
            if let Some(hash) = &data.datum_hash {
                txo.set_data_hash(hash);
            }
            if let Some(datum) = &data.datum {
                txo.set_plutus_data(datum);
            }
            if let Some(sref) = &data.script_ref {
                txo.set_script_ref(sref);
            }
        }
        let mut txos = TransactionOutputs::new();
        txos.add(&txo);
        self.set_txos(&txos);
        Ok(())
    }

    pub fn get_recv_addr(&self) -> Address {
        self.recv_addr.clone()
    }

    pub fn set_txos(&mut self, txos: &TransactionOutputs) {
        self.txos = Some(txos.clone());
    }

    pub fn set_plutus_data(&mut self, pd: TransPData) {
        self.plutus_data = Some(pd);
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Transfer {
    source: Source,
    sinks: Vec<Sink>,
    txos: Option<TransactionOutputs>,
    txis: Option<TransactionInputs>,
}

impl Transfer {
    pub fn new(source: &Source, sinks: &Vec<Sink>) -> Self {
        Transfer {
            source: source.to_owned(),
            sinks: sinks.to_owned(),
            txos: None,
            txis: None,
        }
    }

    pub fn get_source(&self) -> Source {
        self.source.clone()
    }

    pub fn balance(&mut self, wallet: TransWallet) -> Result<(), TransferError> {
        if self.source.pay_value.is_none() {
            return Err(TransferError::SourceNoPaymentValueSet);
        }

        // self.source.set_pay_value(pv);
        self.source.select_txis(&wallet)?;
        log::debug!("Selected inputs...\nSource: {:?}", self.source);
        self.sinks
            .iter_mut()
            .for_each(|n| n.make_outputs().unwrap());
        let in_value = self.source.txiuo.clone().unwrap().calc_total_value()?;
        let out_value = self.sinks.iter().fold(Value::zero(), |mut acc, n| {
            acc = acc.checked_add(&n.value).unwrap();
            acc
        });

        let mut change = in_value.checked_sub(&out_value)?;
        for modificator in &self.source.modificator {
            match modificator {
                TransModificator::Add(v) => {
                    change = change.checked_sub(v)?;
                }
                TransModificator::Sub(v) => {
                    change = change.checked_add(v)?;
                }
            }
        }

        let mut txos = self
            .sinks
            .iter()
            .fold(TransactionOutputs::new(), |mut acc, n| {
                let txos = n.txos.clone().unwrap();
                for m in 0..txos.len() {
                    acc.add(&txos.get(m))
                }
                acc
            });
        let change_txo = TransactionOutput::new(&self.source.pay_addr, &change);
        log::debug!("Change in Transfer: {:?}", change_txo);
        txos.add(&change_txo);
        log::debug!("/nTransactionOutputs in t.balance: {:?}", txos);
        self.txos = Some(txos);
        self.txis = self.source.get_txinputs();
        log::debug!("/n Self.TXOS in t.balance: {:?}", self.txos);

        Ok(())
    }
}
