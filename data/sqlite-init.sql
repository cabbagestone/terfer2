PRAGMA foreign_keys = ON;

create table if not exists node (
    id text primary key not null,
    
    created_at text not null,
    deleted_at text,
);

create table if not exists edge (
    id text primary key not null,
    source text not null,
    target text not null,
    
    created_at text not null,
    deleted_at text,
    
    foreign key (source) references node(id),
    foreign key (target) references node(id)
);

create index if not exists edge_source on edge (source);
create index if not exists edge_target on edge (target);

create table if not exists node_instance (
    id text primary key not null,
    node_id text not null,
    
    value text not null,
    --- 0: created, 1: updated, 2: deleted, 3. restored
    instance_type integer not null,
    
    foreign key (node_id) references node(id)
);
