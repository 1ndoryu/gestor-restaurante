/* [D26d] Marca «Inventariable» de BDP (`Is_Inventoriable` en ExportArticles).
 * NULL = BDP aún no la ha informado; solo `FALSE` impide el traspaso de stock. */
ALTER TABLE bdp_article_map
    ADD COLUMN inventariable BOOLEAN NULL;
