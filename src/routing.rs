use actix_web::web::ServiceConfig;
use crate::{api::*, views::*, hosts::*, instructors::*};

pub fn config_eng_paths(cfg: &mut ServiceConfig) {
    cfg.service(add_eng);
    cfg.service(get_engs);
    cfg.service(edit_eng);
    cfg.service(delete_eng);
}

pub fn config_view_paths(cfg: &mut ServiceConfig) {
    cfg.service(index_root);
    cfg.service(index);
    cfg.service(new_engagement_root);
    cfg.service(new_engagement);
    cfg.service(manage);
}

pub fn config_ins_paths(cfg: &mut ServiceConfig) {
    cfg.service(add_instructor);
    cfg.service(get_instructors);
    cfg.service(delete_instructor);
}

pub fn config_hosts_paths(cfg: &mut ServiceConfig) {
    cfg.service(add_host);
    cfg.service(get_hosts);
    cfg.service(delete_host);
}