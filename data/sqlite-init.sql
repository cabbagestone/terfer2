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
    
    --- Foreign Keys
    foreign key (source) references node(id),
    foreign key (target) references node(id)
);

create index if not exists edge_source on edge (source);
create index if not exists edge_target on edge (target);
