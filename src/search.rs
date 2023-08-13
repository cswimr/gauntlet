use std::thread;
use deno_core::anyhow;
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, Searcher};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;

pub struct SearchIndex {
    index: Index,
    index_reader: IndexReader,
    index_writer: IndexWriter,

    entrypoint_name: Field,
    entrypoint_id: Field,
    plugin_name: Field,
    plugin_id: Field,
}

impl SearchIndex {
    pub fn create_index() -> tantivy::Result<Self> {
        let schema = {
            let mut schema_builder = Schema::builder();

            schema_builder.add_text_field("entrypoint_name", TEXT | STORED);
            schema_builder.add_text_field("entrypoint_id", STORED);
            schema_builder.add_text_field("plugin_name", TEXT | STORED);
            schema_builder.add_text_field("plugin_id", STORED);

            schema_builder.build()
        };

        let entrypoint_name = schema.get_field("entrypoint_name").unwrap();
        let entrypoint_id = schema.get_field("entrypoint_id").unwrap();
        let plugin_name = schema.get_field("plugin_name").unwrap();
        let plugin_id = schema.get_field("plugin_id").unwrap();


        let index = Index::create_in_ram(schema.clone());

        let index_writer = index.writer(50_000_000)?;

        let index_reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()?;

        Ok(Self {
            index,
            index_reader,
            index_writer,
            entrypoint_name,
            entrypoint_id,
            plugin_name,
            plugin_id,
        })
    }

    pub fn add_entries(&mut self, entries: Vec<SearchItem>) -> tantivy::Result<()> {
        let index_writer = &mut self.index_writer;

        for entry in entries {
            index_writer.add_document(doc!(
                self.entrypoint_name => entry.entrypoint_name,
                self.entrypoint_id => entry.entrypoint_id,
                self.plugin_name => entry.plugin_name,
                self.plugin_id => entry.plugin_id,
            ))?;
        }

        index_writer.commit()?;

        thread::sleep(std::time::Duration::from_secs(1)); // FIXME this shouldn't be needed because commit blocks, maybe inmemory index has race condition?
        println!("{:?}", self.index_reader.searcher().num_docs()); // shouldn't return 0

        Ok(())
    }

    pub fn create_handle(&self) -> SearchHandle {
        let searcher = self.index_reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![self.entrypoint_name, self.plugin_name]);

        SearchHandle {
            searcher,
            query_parser,
            entrypoint_name: self.entrypoint_name,
            entrypoint_id: self.entrypoint_id,
            plugin_name: self.plugin_name,
            plugin_id: self.plugin_id,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SearchItem {
    pub entrypoint_name: String,
    pub entrypoint_id: String,
    pub plugin_name: String,
    pub plugin_id: String,
}

pub struct SearchHandle {
    searcher: Searcher,
    query_parser: QueryParser,

    entrypoint_name: Field,
    entrypoint_id: Field,
    plugin_name: Field,
    plugin_id: Field,
}

impl SearchHandle {
    pub(crate) fn search(&self, query: &str) -> anyhow::Result<Vec<SearchItem>> {
        let query = self.query_parser.parse_query(query)?;

        let get_str_field = |retrieved_doc: &Document, field: Field| -> anyhow::Result<String> {
            Ok(
                retrieved_doc.get_first(field)
                    .unwrap_or_else(|| panic!("there should be a field with name {:?}", self.searcher.schema().get_field_name(field)))
                    .as_text()
                    .unwrap_or_else(|| panic!("field with name {:?} should contain string", self.searcher.schema().get_field_name(field)))
                    .to_owned()
            )
        };

        let result = self.searcher.search(&query, &TopDocs::with_limit(10))?
            .into_iter()
            .map(|(_score, doc_address)| {
                let retrieved_doc = self.searcher.doc(doc_address)?;

                Ok(SearchItem {
                    entrypoint_name: get_str_field(&retrieved_doc, self.entrypoint_name)?,
                    entrypoint_id: get_str_field(&retrieved_doc, self.entrypoint_id)?,
                    plugin_name: get_str_field(&retrieved_doc, self.plugin_name)?,
                    plugin_id: get_str_field(&retrieved_doc, self.plugin_id)?,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>();

        Ok(result?)
    }
}