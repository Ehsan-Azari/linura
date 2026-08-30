#!/usr/bin/env ruby
# frozen_string_literal: true

require "psych"

ROOT = File.expand_path("..", __dir__)
WORKFLOW_DIR = File.join(ROOT, ".github", "workflows")
RUNNER_EXPRESSION = /\$\{\{\s*runner\./


def contains_runner?(value)
  case value
  when String
    RUNNER_EXPRESSION.match?(value)
  when Array
    value.any? { |item| contains_runner?(item) }
  when Hash
    value.any? { |key, item| contains_runner?(key) || contains_runner?(item) }
  else
    false
  end
end


def find_env_runner_paths(value, path = [], findings = [])
  case value
  when Hash
    value.each do |key, item|
      key_text = key.to_s
      current_path = path + [key_text]
      if key_text == "env" && contains_runner?(item)
        findings << current_path.join(".")
      end
      find_env_runner_paths(item, current_path, findings)
    end
  when Array
    value.each_with_index do |item, index|
      find_env_runner_paths(item, path + [index.to_s], findings)
    end
  end
  findings
end


def parse_yaml(text, label)
  Psych.safe_load(text, aliases: true, filename: label)
rescue Psych::SyntaxError => error
  raise "invalid YAML in #{label}: #{error.message}"
end


def assert_self_tests!
  rejected = [
    <<~YAML,
      jobs:
        test:
          env : { PROOF_DIR: "${{ runner.temp }}/proof" }
    YAML
    <<~YAML,
      jobs:
        test:
          "env": { PROOF_DIR: "${{ runner.temp }}/proof" }
    YAML
    <<~YAML,
      shared: &shared
        PROOF_DIR: "${{ runner.temp }}/proof"
      jobs:
        test:
          env: *shared
    YAML
    <<~YAML,
      jobs:
        test:
          steps:
            - env:
                PROOF_DIR: "${{ runner.temp }}/proof"
              run: echo ok
    YAML
  ]

  rejected.each_with_index do |yaml, index|
    findings = find_env_runner_paths(parse_yaml(yaml, "self-test-rejected-#{index + 1}"))
    raise "self-test failed to reject env runner expression #{index + 1}" if findings.empty?
  end

  accepted = <<~YAML
    jobs:
      test:
        steps:
          - name: Generate config
            run: |
              cat > config.yml <<'EOF'
              env:
                FOO: "${{ runner.temp }}/proof"
              EOF
          - name: Valid non-env runner context
            uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a
            with:
              path: "${{ runner.temp }}/proof"
  YAML
  findings = find_env_runner_paths(parse_yaml(accepted, "self-test-accepted"))
  raise "self-test falsely rejected scalar/non-env runner expression: #{findings.join(', ')}" unless findings.empty?
end


def main
  assert_self_tests!
  failures = []

  Dir.glob(File.join(WORKFLOW_DIR, "*.{yml,yaml}")).sort.each do |workflow|
    relative = workflow.delete_prefix("#{ROOT}/")
    begin
      document = parse_yaml(File.read(workflow, encoding: "UTF-8"), relative)
      find_env_runner_paths(document).each do |path|
        failures << "runner context is forbidden in env mapping: #{relative}:#{path}"
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
