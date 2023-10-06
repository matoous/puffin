<div align="center">

# Puffin

Experimental implementation of code search based on n-grams

</div>

Based on [Regular Expression Matching with a Trigram Index](https://swtch.com/~rsc/regexp/regexp4.html) by Russ Cox, [The technology behind GitHub’s new code search](https://github.blog/2023-02-06-the-technology-behind-githubs-new-code-search/), and heavily inspired by the Golang implementation [sourcegraph/zoekt](https://github.com/sourcegraph/zoekt).

## How it works

### Indexing

### Search

1. Parse the query into abstract tree. We use [pest](https://pest.rs/) for which we defined [PEG grammar](/src/search.proto).[^peg] GitHub example:
  ```
  And(
      Owner("rails"),
      LanguageID(326),
      Regex("arguments?"),
      Or(
          RepoIDs(...),
          PublicRepo(),
      ),
  )    
  ```

2. Transform the query into a match tree. Match tree is used to evaluate documents. See [matchtree.go](https://github.com/sourcegraph/zoekt/blob/main/matchtree.go) for example, or GitHub's example:
  ```
  and(
    owners_iter("rails"),
    languages_iter(326),
    or(
      and(
        content_grams_iter("arg"),
        content_grams_iter("rgu"),
        content_grams_iter("gum"),
        or(
          and(
           content_grams_iter("ume"),
           content_grams_iter("ment")
          )
          content_grams_iter("uments"),
        )
      ),
      or(paths_grams_iter…)
      or(symbols_grams_iter…)
    ), 
    …
  )   
  ```

3. Evaluate the documents in each shard against the match tree. If correctly optimized the matchtree should work from most specific to least specific queries (i.e. reduce the search set as much as possible early on). Internally this works by having an iterator chain. The search iterates ower documents as they get filtered by the match tree and collects only those documents that pass the whole chain.

4. Return the matches as a result.

[^peg]: [Parsing expression grammar (PEG)](https://en.wikipedia.org/wiki/Parsing_expression_grammar)
