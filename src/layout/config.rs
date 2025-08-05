pub trait ConfigValue {
    fn as_integer(&self) -> Option<i64>;
    fn as_float(&self) -> Option<f64>;
    fn as_string(&self) -> Option<&str>;
    fn as_bool(&self) -> Option<bool>;
    fn as_table(&self) -> Option<&dyn ConfigTable<Value = Self>>;
}

impl ConfigValue for toml::Value {
    fn as_integer(&self) -> Option<i64> {
        match self {
            toml::Value::Integer(n) => Some(*n),
            _ => None,
        }
    }

    fn as_float(&self) -> Option<f64> {
        match self {
            toml::Value::Float(f) => Some(*f),
            toml::Value::Integer(n) => Some(*n as f64),
            _ => None,
        }
    }

    fn as_string(&self) -> Option<&str> {
        match self {
            toml::Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            toml::Value::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    fn as_table(&self) -> Option<&dyn ConfigTable<Value = Self>> {
        match self {
            toml::Value::Table(table) => Some(table),
            _ => None,
        }
    }
}
pub trait ConfigTable {
    type Value: ConfigValue;

    fn get(&self, key: &str) -> Option<&Self::Value>;
    fn get_mut(&mut self, key: &str) -> Option<&mut Self::Value>;
    fn contains_key(&self, key: &str) -> bool;
    fn insert(&mut self, key: &str, value: Self::Value);
    fn iter_mut(&mut self) -> Box<dyn Iterator<Item = (&str, &mut Self::Value)> + '_>;
    fn remove(&mut self, key: &str) -> Option<Self::Value>;

    fn from_value(value: Self::Value) -> Option<Self>
    where
        Self: Sized;
}

impl ConfigTable for toml::Table {
    type Value = toml::Value;

    fn get(&self, key: &str) -> Option<&Self::Value> {
        self.get(key)
    }

    fn get_mut(&mut self, key: &str) -> Option<&mut Self::Value> {
        self.get_mut(key)
    }

    fn contains_key(&self, key: &str) -> bool {
        self.contains_key(key)
    }

    fn insert(&mut self, key: &str, value: Self::Value) {
        self.insert(key.to_string(), value);
    }

    fn iter_mut(&mut self) -> Box<dyn Iterator<Item = (&str, &mut Self::Value)> + '_> {
        Box::new(self.iter_mut().map(|(k, v)| (k.as_str(), v)))
    }

    fn remove(&mut self, key: &str) -> Option<Self::Value> {
        self.remove(key)
    }

    fn from_value(value: Self::Value) -> Option<Self> {
        match value {
            toml::Value::Table(table) => Some(table),
            _ => None,
        }
    }
}
