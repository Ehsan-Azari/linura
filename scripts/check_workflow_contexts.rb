#!/usr/bin/env ruby
# frozen_string_literal: true

require "psych"

ROOT = File.expand_path("..", __dir__)
WORKFLOW_DIR = File.join(ROOT, ".github", "workflows")
RUNNER_ROOT = /(?<![A-Za-z0-9_.])runner(?![A-Za-z0-9_])/i

# Targeted regression guard for GitHub Actions context availability.
#
# GitHub does not expose the `runner` context in workflow-level `env` or
# `jobs.<job_id>.env`, because both are evaluated before a runner is available.
# GitHub does expose `runner` in step env, step with, container env, and service
# env. This checker therefore validates only the two schema positions where the
# original Linura release workflows failed; it is intentionally not a complete
# GitHub Actions schema validator.


def expression_bodies(text)
  bodies = []
  cursor = 0

  while (start_index = text.index("${{", cursor))
    index = start_index + 3
    body_start = index
    in_string = false
    closed = false

    while index < text.length
      char = text[index]

      if in_string
        if char == "'" && text[index + 1] == "'"
          index += 2
          next
        end
        in_string = false if char == "'"
        index += 1
        next
      end

      if char == "'"
        in_string = true
        index += 1
        next
      end

      if text[index, 2] == "}}"
        bodies << text[body_start...index]
        cursor = index + 2
        closed = true
        break
      end

      index += 1
    end

    break unless closed
  end

  bodies
end


def strip_expression_strings(expression)
  result = +""
  index = 0
  in_string = false

  while index < expression.length
    char = expression[index]

    if in_string
      if char == "'" && expression[index + 1] == "'"
        result << "  "
        index += 2
        next
      end
      in_string = false if char == "'"
      result << " "
      index += 1
      next
    end

    if char == "'"
      in_string = true
      result << " "
    else
      result << char
    end
    index += 1
  end

  result
end


def contains_runner_reference?(value)
  case value
  when String
    expression_bodies(value).any? do |expression|
      RUNNER_ROOT.match?(strip_expression_strings(expression))
    end
  when Array
    value.any? { |item| contains_runner_reference?(item) }
  when Hash
    value.any? do |key, item|
      contains_runner_reference?(key) || contains_runner_reference?(item)
    end
  else
    false
  end
end


def restricted_env_mappings(document)
  return [] unless document.is_a?(Hash)

  mappings = []
  mappings << ["env", document["env"]] if document.key?("env")

  jobs = document["jobs"]
  if jobs.is_a?(Hash)
    jobs.each do |job_id, job|
      next unless job.is_a?(Hash) && job.key?("env")

      mappings << ["jobs.#{job_id}.env", job["env"]]
    end
  end

  mappings
end


def find_invalid_runner_paths(document)
  restricted_env_mappings(document).filter_map do |path, env_mapping|
    path if contains_runner_reference?(env_mapping)
  end
end


def parse_yaml(text, label)
  Psych.safe_load(text, aliases: true, filename: label)
rescue Psych::SyntaxError => error
  raise "invalid YAML in #{label}: #{error.message}"
end


def assert_rejected!(yaml, label)
  findings = find_invalid_runner_paths(parse_yaml(yaml, label))
  raise "self-test failed to reject #{label}" if findings.empty?
end


def assert_accepted!(yaml, label)
  findings = find_invalid_runner_paths(parse_yaml(yaml, label))
  return if findings.empty?

  raise "self-test falsely rejected #{label}: #{findings.join(', ')}"
end


def assert_self_tests!
  rejected = {
    "workflow-env-dot" => <<~YAML,
      env:
        PROOF_DIR: "${{ runner.temp }}/proof"
      jobs:
        test:
          runs-on: ubuntu-latest
          steps:
            - run: echo ok
    YAML
    "job-env-spaced-key" => <<~YAML,
      jobs:
        test:
          env : { PROOF_DIR: "${{ runner.temp }}/proof" }
          runs-on: ubuntu-latest
          steps:
            - run: echo ok
    YAML
    "job-env-quoted-key" => <<~YAML,
      jobs:
        test:
          "env": { PROOF_DIR: "${{ runner.temp }}/proof" }
          runs-on: ubuntu-latest
          steps:
            - run: echo ok
    YAML
    "job-env-alias" => <<~YAML,
      shared: &shared
        PROOF_DIR: "${{ runner.temp }}/proof"
      jobs:
        test:
          env: *shared
          runs-on: ubuntu-latest
          steps:
            - run: echo ok
    YAML
    "job-env-index" => <<~YAML,
      jobs:
        test:
          env:
            PROOF_DIR: "${{ runner['temp'] }}/proof"
          runs-on: ubuntu-latest
          steps:
            - run: echo ok
    YAML
    "job-env-spaced-index" => <<~YAML,
      jobs:
        test:
          env:
            PROOF_DIR: "${{ runner [ 'temp' ] }}/proof"
          runs-on: ubuntu-latest
          steps:
            - run: echo ok
    YAML
    "job-env-bare-runner" => <<~YAML,
      jobs:
        test:
          env:
            RUNNER_JSON: "${{ toJson(runner) }}"
          runs-on: ubuntu-latest
          steps:
            - run: echo ok
    YAML
  }
  rejected.each { |label, yaml| assert_rejected!(yaml, label) }

  accepted = <<~YAML
    env:
      LITERAL_AFTER_EXPRESSION: "${{ github.ref }} runner.temp"
      STRING_LITERAL_IN_EXPRESSION: "${{ contains(github.ref, 'runner.temp') }}"
      INDEX_LITERAL_IN_EXPRESSION: "${{ github['runner'] }}"
    jobs:
      test:
        env:
          VALID_JOB_VALUE: "${{ github.ref }}"
        runs-on: ubuntu-latest
        container:
          image: ruby:3.4
          env:
            CONTAINER_TMP: "${{ runner.temp }}/container"
        services:
          db:
            image: postgres:18
            env:
              SERVICE_ARCH: "${{ runner.arch }}"
        steps:
          - name: Runner is valid in step env
            env:
              STEP_TMP: "${{ runner.temp }}/step"
            run: echo "$STEP_TMP"
          - name: Action input named env is not workflow env
            uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a
            with:
              env: "${{ runner.temp }}/input"
              path: "${{ runner['temp'] }}/proof"
          - name: Env-like text inside a run scalar is not workflow structure
            run: |
              cat > config.yml <<'EOF'
              env:
                FOO: "${{ runner.temp }}/proof"
              EOF
  YAML
  assert_accepted!(accepted, "runner-aware-valid-schema-positions")
end


def main
  assert_self_tests!
  failures = []

  Dir.glob(File.join(WORKFLOW_DIR, "*.{yml,yaml}")).sort.each do |workflow|
    relative = workflow.delete_prefix("#{ROOT}/")
    begin
      document = parse_yaml(File.read(workflow, encoding: "UTF-8"), relative)
      find_invalid_runner_paths(document).each do |path|
        failures << "runner context is unavailable at #{relative}:#{path}"
      end
    rescue StandardError => error
      failures << error.message
    end
  end

  unless failures.empty?
    failures.each { |failure| warn "ERROR: #{failure}" }
    return 1
  end

  puts "GitHub workflow context checks passed"
  0
end

exit(main)
