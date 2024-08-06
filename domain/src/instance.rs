use jiff::Zoned;

pub(crate) struct Instance {
    saved_at: Zoned,
    instance_type: InstanceType,
    value: String
}

impl Instance {
    pub(crate) fn new_created(value: String) -> Instance {
        Instance {
            saved_at: Zoned::now(),
            instance_type: InstanceType::Created,
            value
        }
    }

    pub(crate) fn new_updated(value: String) -> Instance {
        Instance {
            saved_at: Zoned::now(),
            instance_type: InstanceType::Updated,
            value
        }
    }

    pub(crate) fn deleted_child(&self) -> Instance {
        Instance {
            saved_at: Zoned::now(),
            instance_type: InstanceType::Deleted,
            value: self.value.clone()
        }
    }

    pub(crate) fn restored_child(&self) -> Instance {
        Instance {
            saved_at: Zoned::now(),
            instance_type: InstanceType::Restored,
            value: self.value.clone()
        }
    }
    
    pub(crate) fn value(&self) -> &str {
        &self.value
    }
}

enum InstanceType {
    Created,
    Deleted,
    Restored,
    Updated
}