export interface CatalogMap {
    [key: string]: CatalogNode;
}

export type CatalogNode = string | CatalogMap;

export type Catalog = CatalogMap;
