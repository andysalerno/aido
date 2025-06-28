# aido

"AI do". Like "sudo". Get it??

## Modes

One-off chat:

```
$ aido 'write a commit message'
```

Recipes / aliases:

```
$ aido run commit
```

(where `commit.prompt` exists in `~/.config/aido/prompts/`)

Continue the last conversation:

```
$ aido 'what's the weather in Seattle?'
It's rainy and 72F in Seattle. Do you want the weather for next week as well?
$ aido -c 'yes'
Next week will be...
```

(alternatively, perhaps the LLM should decide if the chat is over or if it wants one more user input... but need a way to force 1-turn outputs for scripts)

## Configuration

```
$ aido show-config
...prints the current config content
```

```
$ aido show-config-path
...prints the path to the config file being used
```

```
$ aido set-model mistralai/mistral-small-3.2-24b-instruct
...updates the configured model
```

## Tools & MCP
(try to emulate docker/podman CLI patterns)

```
$ aido tools ls
...shows all tools, whether enabled, whether confirmation required
```

## Dependencies

There are dependencies in this project that would be ideal to remove over time.

- async-openai
- tokio (only needed to run async-openai)