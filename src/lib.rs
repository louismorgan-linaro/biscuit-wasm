use std::collections::HashMap;

use biscuit_auth as biscuit;
use serde::{de::Visitor, Deserialize};
use wasm_bindgen::prelude::*;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

/// a Biscuit token
///
/// it can produce an attenuated or sealed token, or be used
/// in an authorizer along with Datalog policies
#[wasm_bindgen]
pub struct Biscuit(biscuit::Biscuit);

#[wasm_bindgen]
impl Biscuit {
    /// Creates a BiscuitBuilder
    ///
    /// the builder can then create a new token with a root key
    pub fn builder() -> BiscuitBuilder {
        BiscuitBuilder::new()
    }

    /// Creates an attenuated token by adding the block generated by the BlockBuilder
    #[wasm_bindgen(js_name = appendBlock)]
    pub fn append(&self, block: BlockBuilder) -> Result<Biscuit, JsValue> {
        let keypair = KeyPair::new();
        Ok(Biscuit(
            self.0
                .append_with_keypair(&keypair.0, block.0)
                .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?,
        ))
    }

    /// Creates an authorizer from the token
    #[wasm_bindgen(js_name = getAuthorizer)]
    pub fn authorizer(&self) -> Result<Authorizer, JsValue> {
        Ok(Authorizer {
            token: Some(self.0.clone()),
            ..Authorizer::default()
        })
    }

    /// Seals the token
    ///
    /// A sealed token cannot be attenuated
    #[wasm_bindgen(js_name = sealToken)]
    pub fn seal(&self) -> Result<Biscuit, JsValue> {
        Ok(Biscuit(
            self.0
                .seal()
                .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?,
        ))
    }

    /// Deserializes a token from raw data
    ///
    /// This will check the signature using the root key
    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(data: &[u8], root: &PublicKey) -> Result<Biscuit, JsValue> {
        Ok(Biscuit(
            biscuit::Biscuit::from(data, root.0)
                .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?,
        ))
    }

    /// Deserializes a token from URL safe base 64 data
    ///
    /// This will check the signature using the root key
    #[wasm_bindgen(js_name = fromBase64)]
    pub fn from_base64(data: &str, root: &PublicKey) -> Result<Biscuit, JsValue> {
        Ok(Biscuit(
            biscuit::Biscuit::from_base64(data, root.0)
                .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?,
        ))
    }

    /// Serializes to raw data
    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Result<Box<[u8]>, JsValue> {
        Ok(self
            .0
            .to_vec()
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?
            .into_boxed_slice())
    }

    /// Serializes to URL safe base 64 data
    #[wasm_bindgen(js_name = toBase64)]
    pub fn to_base64(&self) -> Result<String, JsValue> {
        self.0
            .to_base64()
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }

    /// Returns the list of revocation identifiers, encoded as URL safe base 64
    #[wasm_bindgen(js_name = getRevocationIdentifiers)]
    pub fn revocation_identifiers(&self) -> Box<[JsValue]> {
        let ids: Vec<_> = self
            .0
            .revocation_identifiers()
            .into_iter()
            .map(|id| base64::encode_config(id, base64::URL_SAFE).into())
            .collect();
        ids.into_boxed_slice()
    }

    /// Returns the number of blocks in the token
    #[wasm_bindgen(js_name = countBlocks)]
    pub fn block_count(&self) -> usize {
        self.0.block_count()
    }

    /// Prints a block's content as Datalog code
    #[wasm_bindgen(js_name = getBlockSource)]
    pub fn block_source(&self, index: usize) -> Result<String, JsValue> {
        self.0
            .print_block_source(index)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }

    /// Creates a third party request
    #[wasm_bindgen(js_name = getThirdPartyRequest)]
    pub fn third_party_request(&self) -> Result<ThirdPartyRequest, JsValue> {
        Ok(ThirdPartyRequest(
            self.0
                .third_party_request()
                .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?,
        ))
    }

    /// Appends a third party block and returns a new token
    #[wasm_bindgen(js_name = appendThirdPartyBlock)]
    pub fn append_third_party(
        &self,
        external_key: PublicKey,
        block: ThirdPartyBlock,
    ) -> Result<Biscuit, JsValue> {
        Ok(Biscuit(
            self.0
                .append_third_party(external_key.0, block.0)
                .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?,
        ))
    }
}

/// The Authorizer verifies a request according to its policies and the provided token
#[wasm_bindgen]
#[derive(Default)]
pub struct Authorizer {
    token: Option<biscuit::Biscuit>,
    facts: Vec<biscuit::builder::Fact>,
    rules: Vec<biscuit::builder::Rule>,
    checks: Vec<biscuit::builder::Check>,
    policies: Vec<biscuit::builder::Policy>,
}

#[wasm_bindgen]
impl Authorizer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Authorizer {
        Authorizer::default()
    }

    #[wasm_bindgen(js_name = addToken)]
    pub fn add_token(&mut self, token: Biscuit) {
        self.token = Some(token.0);
    }

    /// Adds a Datalog fact
    #[wasm_bindgen(js_name = addFact)]
    pub fn add_fact(&mut self, fact: Fact) -> Result<(), JsValue> {
        self.facts.push(fact.0);
        Ok(())
    }

    /// Adds a Datalog rule
    #[wasm_bindgen(js_name = addRule)]
    pub fn add_rule(&mut self, rule: Rule) -> Result<(), JsValue> {
        self.rules.push(rule.0);
        Ok(())
    }

    /// Adds a check
    ///
    /// All checks, from authorizer and token, must be validated to authorize the request
    #[wasm_bindgen(js_name = addCheck)]
    pub fn add_check(&mut self, check: Check) -> Result<(), JsValue> {
        self.checks.push(check.0);
        Ok(())
    }

    /// Adds a policy
    ///
    /// The authorizer will test all policies in order of addition and stop at the first one that
    /// matches. If it is a "deny" policy, the request fails, while with an "allow" policy, it will
    /// succeed
    #[wasm_bindgen(js_name = addPolicy)]
    pub fn add_policy(&mut self, policy: Policy) -> Result<(), JsValue> {
        self.policies.push(policy.0);
        Ok(())
    }

    /// Adds facts, rules, checks and policies as one code block
    #[wasm_bindgen(js_name = addCode)]
    pub fn add_code(&mut self, source: &str) -> Result<(), JsValue> {
        let source_result = biscuit::parser::parse_source(source).map_err(|e| {
            let e2: biscuit_parser::error::LanguageError = e.into();
            let e: biscuit::error::Token = e2.into();
            serde_wasm_bindgen::to_value(&e).unwrap()
        })?;

        for (_, fact) in source_result.facts.into_iter() {
            self.facts.push(fact.into());
        }

        for (_, rule) in source_result.rules.into_iter() {
            self.rules.push(rule.into());
        }

        for (_, check) in source_result.checks.into_iter() {
            self.checks.push(check.into());
        }

        for (_, policy) in source_result.policies.into_iter() {
            self.policies.push(policy.into());
        }

        Ok(())
    }

    /// Runs the authorization checks and policies
    ///
    /// Returns the index of the matching allow policy, or an error containing the matching deny
    /// policy or a list of the failing checks
    #[wasm_bindgen(js_name = authorize)]
    pub fn authorize(&self) -> Result<usize, JsValue> {
        let mut authorizer = match &self.token {
            Some(token) => token
                .authorizer()
                .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?,
            None => biscuit::Authorizer::new(),
        };

        for fact in self.facts.iter() {
            authorizer
                .add_fact(fact.clone())
                .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?;
        }
        for rule in self.rules.iter() {
            authorizer
                .add_rule(rule.clone())
                .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?;
        }
        for check in self.checks.iter() {
            authorizer
                .add_check(check.clone())
                .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?;
        }
        for policy in self.policies.iter() {
            authorizer
                .add_policy(policy.clone())
                .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?;
        }

        authorizer
            .authorize()
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }
}

/// Creates a block to attenuate a token
#[wasm_bindgen]
pub struct ThirdPartyRequest(biscuit::ThirdPartyRequest);

#[wasm_bindgen]
impl ThirdPartyRequest {
    /// Deserializes a third party request from raw data
    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(data: &[u8]) -> Result<ThirdPartyRequest, JsValue> {
        Ok(ThirdPartyRequest(
            biscuit::ThirdPartyRequest::deserialize(data)
                .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?,
        ))
    }

    /// Deserializes a third party request from URL safe base 64 data
    ///
    /// This will check the signature using the root key
    #[wasm_bindgen(js_name = fromBase64)]
    pub fn from_base64(data: &str) -> Result<ThirdPartyRequest, JsValue> {
        Ok(ThirdPartyRequest(
            biscuit::ThirdPartyRequest::deserialize_base64(data)
                .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?,
        ))
    }

    /// Serializes to raw data
    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Result<Box<[u8]>, JsValue> {
        Ok(self
            .0
            .serialize()
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?
            .into_boxed_slice())
    }

    /// Serializes to URL safe base 64 data
    #[wasm_bindgen(js_name = toBase64)]
    pub fn to_base64(&self) -> Result<String, JsValue> {
        self.0
            .serialize_base64()
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }

    /// creates a ThirdPartyBlock from a BlockBuilder and the
    /// third party service's private key
    #[wasm_bindgen(js_name = createBlock)]
    pub fn create_block(
        self,
        private_key: &PrivateKey,
        block_builder: BlockBuilder,
    ) -> Result<ThirdPartyBlock, JsValue> {
        Ok(ThirdPartyBlock(
            self.0
                .create_block(&private_key.0, block_builder.0)
                .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?,
        ))
    }
}

#[wasm_bindgen]
pub struct ThirdPartyBlock(biscuit::ThirdPartyBlock);

#[wasm_bindgen]
impl ThirdPartyBlock {
    /// Deserializes a third party request from raw data
    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(data: &[u8]) -> Result<ThirdPartyRequest, JsValue> {
        Ok(ThirdPartyRequest(
            biscuit::ThirdPartyRequest::deserialize(data)
                .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?,
        ))
    }

    /// Deserializes a third party request from URL safe base 64 data
    ///
    /// This will check the signature using the root key
    #[wasm_bindgen(js_name = fromBase64)]
    pub fn from_base64(data: &str) -> Result<ThirdPartyRequest, JsValue> {
        Ok(ThirdPartyRequest(
            biscuit::ThirdPartyRequest::deserialize_base64(data)
                .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?,
        ))
    }

    /// Serializes to raw data
    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Result<Box<[u8]>, JsValue> {
        Ok(self
            .0
            .serialize()
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?
            .into_boxed_slice())
    }

    /// Serializes to URL safe base 64 data
    #[wasm_bindgen(js_name = toBase64)]
    pub fn to_base64(self) -> Result<String, JsValue> {
        self.0
            .serialize_base64()
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }
}

/// Creates a token
#[wasm_bindgen]
pub struct BiscuitBuilder(biscuit::builder::BiscuitBuilder);

#[wasm_bindgen]
impl BiscuitBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> BiscuitBuilder {
        BiscuitBuilder(biscuit::builder::BiscuitBuilder::new())
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self, root: &PrivateKey) -> Result<Biscuit, JsValue> {
        let keypair = biscuit_auth::KeyPair::from(&root.0);

        let mut rng = make_rng();
        Ok(Biscuit(
            self.0
                .build_with_rng(&keypair, biscuit::datalog::SymbolTable::default(), &mut rng)
                .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?,
        ))
    }

    /// adds the content of an existing `BlockBuilder`
    pub fn merge(&mut self, other: BlockBuilder) {
        self.0.merge(other.0)
    }

    /// Adds a Datalog fact
    #[wasm_bindgen(js_name = addFact)]
    pub fn add_fact(&mut self, fact: Fact) -> Result<(), JsValue> {
        self.0
            .add_fact(fact.0)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }

    /// Adds a Datalog rule
    #[wasm_bindgen(js_name = addRule)]
    pub fn add_rule(&mut self, rule: Rule) -> Result<(), JsValue> {
        self.0
            .add_rule(rule.0)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }

    /// Adds a check
    ///
    /// All checks, from authorizer and token, must be validated to authorize the request
    #[wasm_bindgen(js_name = addCheck)]
    pub fn add_check(&mut self, check: Check) -> Result<(), JsValue> {
        self.0
            .add_check(check.0)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }

    /// Adds facts, rules, checks and policies as one code block
    #[wasm_bindgen(js_name = addCode)]
    pub fn add_code(&mut self, source: &str) -> Result<(), JsValue> {
        self.0
            .add_code(source)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }

    /// Adds facts, rules, checks and policies as one code block
    #[wasm_bindgen(js_name = addCodeWithParameters)]
    pub fn add_code_with_parameters(
        &mut self,
        source: &str,
        parameters: JsValue,
        scope_parameters: JsValue,
    ) -> Result<(), JsValue> {
        let parameters: HashMap<String, Term> = serde_wasm_bindgen::from_value(parameters).unwrap();

        let parameters = parameters
            .into_iter()
            .map(|(k, t)| (k, t.0))
            .collect::<HashMap<_, _>>();

        let scope_parameters: HashMap<String, PublicKey> =
            serde_wasm_bindgen::from_value(scope_parameters).unwrap();
        let scope_parameters = scope_parameters
            .into_iter()
            .map(|(k, p)| (k, p.0))
            .collect::<HashMap<_, _>>();

        self.0
            .add_code_with_params(source, parameters, scope_parameters)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }
}

/// Creates a block to attenuate a token
#[wasm_bindgen]
pub struct BlockBuilder(biscuit::builder::BlockBuilder);

#[wasm_bindgen]
impl BlockBuilder {
    /// creates a BlockBuilder
    ///
    /// the builder can then be given to the token's append method to create an attenuated token
    #[wasm_bindgen(constructor)]
    pub fn new() -> BlockBuilder {
        BlockBuilder(biscuit::builder::BlockBuilder::new())
    }

    /// Adds a Datalog fact
    #[wasm_bindgen(js_name = addFact)]
    pub fn add_fact(&mut self, fact: Fact) -> Result<(), JsValue> {
        self.0
            .add_fact(fact.0)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }

    /// Adds a Datalog rule
    #[wasm_bindgen(js_name = addRule)]
    pub fn add_rule(&mut self, rule: Rule) -> Result<(), JsValue> {
        self.0
            .add_rule(rule.0)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }

    /// Adds a check
    ///
    /// All checks, from authorizer and token, must be validated to authorize the request
    #[wasm_bindgen(js_name = addCheck)]
    pub fn add_check(&mut self, check: Check) -> Result<(), JsValue> {
        self.0
            .add_check(check.0)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }

    /// Adds facts, rules, checks and policies as one code block
    #[wasm_bindgen(js_name = addCode)]
    pub fn add_code(&mut self, source: &str) -> Result<(), JsValue> {
        self.0
            .add_code(source)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }

    /// Adds facts, rules, checks and policies as one code block
    #[wasm_bindgen(js_name = addCodeWithParameters)]
    pub fn add_code_with_parameters(
        &mut self,
        source: &str,
        parameters: JsValue,
        scope_parameters: JsValue,
    ) -> Result<(), JsValue> {
        let parameters: HashMap<String, Term> = serde_wasm_bindgen::from_value(parameters).unwrap();

        let parameters = parameters
            .into_iter()
            .map(|(k, t)| (k, t.0))
            .collect::<HashMap<_, _>>();

        let scope_parameters: HashMap<String, PublicKey> =
            serde_wasm_bindgen::from_value(scope_parameters).unwrap();
        let scope_parameters = scope_parameters
            .into_iter()
            .map(|(k, p)| (k, p.0))
            .collect::<HashMap<_, _>>();

        self.0
            .add_code_with_params(source, parameters, scope_parameters)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }
}

#[wasm_bindgen]
pub struct Fact(biscuit::builder::Fact);

#[wasm_bindgen]
impl Fact {
    #[wasm_bindgen(js_name = fromString)]
    pub fn from_str(source: &str) -> Result<Fact, JsValue> {
        source
            .try_into()
            .map(Fact)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }

    #[wasm_bindgen(js_name = set)]
    pub fn set(&mut self, name: &str, value: JsValue) -> Result<(), JsValue> {
        let value = js_to_term(value)?;

        self.0
            .set(name, value)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }
}

#[wasm_bindgen]
pub struct Rule(biscuit::builder::Rule);

#[wasm_bindgen]
impl Rule {
    #[wasm_bindgen(js_name = fromString)]
    pub fn from_str(source: &str) -> Result<Rule, JsValue> {
        source
            .try_into()
            .map(Rule)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }

    #[wasm_bindgen(js_name = set)]
    pub fn set(&mut self, name: &str, value: JsValue) -> Result<(), JsValue> {
        let value = js_to_term(value)?;

        self.0
            .set(name, value)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }
}

#[wasm_bindgen]
pub struct Check(biscuit::builder::Check);

#[wasm_bindgen]
impl Check {
    #[wasm_bindgen(js_name = fromString)]
    pub fn from_str(source: &str) -> Result<Check, JsValue> {
        source
            .try_into()
            .map(Check)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }

    #[wasm_bindgen(js_name = set)]
    pub fn set(&mut self, name: &str, value: JsValue) -> Result<(), JsValue> {
        let value = js_to_term(value)?;

        self.0
            .set(name, value)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }
}

#[wasm_bindgen]
pub struct Policy(biscuit::builder::Policy);

#[wasm_bindgen]
impl Policy {
    #[wasm_bindgen(js_name = fromString)]
    pub fn from_str(source: &str) -> Result<Policy, JsValue> {
        source
            .try_into()
            .map(Policy)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }

    #[wasm_bindgen(js_name = set)]
    pub fn set(&mut self, name: &str, value: JsValue) -> Result<(), JsValue> {
        let value = js_to_term(value)?;

        self.0
            .set(name, value)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }
}

fn js_to_term(value: JsValue) -> Result<biscuit::builder::Term, JsValue> {
    serde_wasm_bindgen::from_value(value)
        .map(|t: Term| t.0)
        .map_err(|e| serde_wasm_bindgen::to_value(&e.to_string()).unwrap())
}

struct Term(biscuit::builder::Term);

impl<'de> Deserialize<'de> for Term {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(TermVisitor)
    }
}

struct TermVisitor;

impl<'de> Visitor<'de> for TermVisitor {
    type Value = Term;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a datalog term")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Term(biscuit::builder::boolean(v)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Term(biscuit::builder::int(value)))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Term(biscuit::builder::Term::Str(v)))
    }
}

/// A pair of public and private key
#[wasm_bindgen]
pub struct KeyPair(biscuit::KeyPair);

#[wasm_bindgen]
impl KeyPair {
    #[wasm_bindgen(constructor)]
    pub fn new() -> KeyPair {
        let mut rng = make_rng();
        KeyPair(biscuit::KeyPair::new_with_rng(&mut rng))
    }

    #[wasm_bindgen(js_name = fromPrivateKey)]
    pub fn from(key: PrivateKey) -> Self {
        KeyPair(biscuit::KeyPair::from(&key.0))
    }

    #[wasm_bindgen(js_name = getPublicKey)]
    pub fn public(&self) -> PublicKey {
        PublicKey(self.0.public())
    }

    #[wasm_bindgen(js_name = getPrivateKey)]
    pub fn private(&self) -> PrivateKey {
        PrivateKey(self.0.private())
    }
}

/// Public key
#[wasm_bindgen]
pub struct PublicKey(biscuit::PublicKey);

#[wasm_bindgen]
impl PublicKey {
    /// Serializes a public key to raw bytes
    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self, out: &mut [u8]) -> Result<(), JsValue> {
        if out.len() != 32 {
            return Err(serde_wasm_bindgen::to_value(&biscuit::error::Token::Format(
                biscuit::error::Format::InvalidKeySize(out.len()),
            ))
            .unwrap());
        }

        out.copy_from_slice(&self.0.to_bytes());
        Ok(())
    }

    /// Serializes a public key to a hexadecimal string
    #[wasm_bindgen(js_name = toString)]
    pub fn to_hex(&self) -> String {
        hex::encode(self.0.to_bytes())
    }

    /// Deserializes a public key from raw bytes
    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(data: &[u8]) -> Result<PublicKey, JsValue> {
        let key = biscuit_auth::PublicKey::from_bytes(data)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?;
        Ok(PublicKey(key))
    }

    /// Deserializes a public key from a hexadecimal string
    #[wasm_bindgen(js_name = fromString)]
    pub fn from_hex(data: &str) -> Result<PublicKey, JsValue> {
        let data = hex::decode(data).map_err(|e| {
            serde_wasm_bindgen::to_value(&biscuit::error::Token::Format(
                biscuit::error::Format::InvalidKey(format!(
                    "could not deserialize hex encoded key: {}",
                    e
                )),
            ))
            .unwrap()
        })?;
        let key = biscuit_auth::PublicKey::from_bytes(&data)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?;
        Ok(PublicKey(key))
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(PublicKeyVisitor)
    }
}

struct PublicKeyVisitor;

impl<'de> Visitor<'de> for PublicKeyVisitor {
    type Value = PublicKey;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a public key")
    }

    fn visit_string<E>(self, s: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match s.strip_prefix("ed25519/") {
            None => Err(E::custom(
                "expected a public key of the format `ed25519/<hex>`".to_string(),
            )),
            Some(s) => match biscuit::PublicKey::from_bytes_hex(s) {
                Ok(pk) => Ok(PublicKey(pk)),
                Err(e) => Err(E::custom(format!("could not parse public key: {}", e))),
            },
        }
    }
}

#[wasm_bindgen]
pub struct PrivateKey(biscuit::PrivateKey);

#[wasm_bindgen]
impl PrivateKey {
    /// Serializes a private key to raw bytes
    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self, out: &mut [u8]) -> Result<(), JsValue> {
        if out.len() != 32 {
            return Err(serde_wasm_bindgen::to_value(&biscuit::error::Token::Format(
                biscuit::error::Format::InvalidKeySize(out.len()),
            ))
            .unwrap());
        }

        out.copy_from_slice(&self.0.to_bytes());
        Ok(())
    }

    /// Serializes a private key to a hexadecimal string
    #[wasm_bindgen(js_name = toString)]
    pub fn to_hex(&self) -> String {
        hex::encode(self.0.to_bytes())
    }

    /// Deserializes a private key from raw bytes
    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(data: &[u8]) -> Result<PrivateKey, JsValue> {
        let key = biscuit_auth::PrivateKey::from_bytes(data)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?;
        Ok(PrivateKey(key))
    }

    /// Deserializes a private key from a hexadecimal string
    #[wasm_bindgen(js_name = fromString)]
    pub fn from_hex(data: &str) -> Result<PrivateKey, JsValue> {
        let data = hex::decode(data).map_err(|e| {
            serde_wasm_bindgen::to_value(&biscuit::error::Token::Format(
                biscuit::error::Format::InvalidKey(format!(
                    "could not deserialize hex encoded key: {}",
                    e
                )),
            ))
            .unwrap()
        })?;
        let key = biscuit_auth::PrivateKey::from_bytes(&data)
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())?;
        Ok(PrivateKey(key))
    }
}

fn make_rng() -> rand::rngs::StdRng {
    let mut data = [0u8; 8];
    getrandom::getrandom(&mut data[..]).unwrap();
    rand::SeedableRng::seed_from_u64(u64::from_le_bytes(data))
}

#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen(start)]
pub fn init() {
    wasm_logger::init(wasm_logger::Config::default());
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    log("biscuit-wasm loading")
}
