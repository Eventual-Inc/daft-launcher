use crate::StrRef;

pub fn default_region() -> StrRef {
    "us-west-2".into()
}

pub fn default_ssh_user() -> StrRef {
    "ec2-user".into()
}

pub fn light_image_id() -> StrRef {
    "ami-07c5ecd8498c59db5".into()
}

pub fn light_instance_type() -> StrRef {
    "t2.nano".into()
}

pub fn normal_image_id() -> StrRef {
    "ami-07dcfc8123b5479a8".into()
}

pub fn normal_instance_type() -> StrRef {
    "m7g.medium".into()
}

pub const DEFAULT_NUMBER_OF_WORKERS: usize = 2;
