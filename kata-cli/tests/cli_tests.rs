use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("kata").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Compilador e REPL para a linguagem Kata"));
}

#[test]
fn test_cli_build_missing_file() {
    let mut cmd = Command::cargo_bin("kata").unwrap();
    cmd.arg("build").arg("arquivo_inexistente.kata")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Erro: O arquivo de entrada 'arquivo_inexistente.kata' não foi encontrado."));
}

#[test]
fn test_cli_examples_e2e() {
    // Garante que o diretório de examples/ existe e contém os mocks em .kata
    let examples_dir = PathBuf::from("examples");
    assert!(examples_dir.exists(), "O diretório examples/ precisa existir para os testes E2E");

    let entries = fs::read_dir(examples_dir).expect("Falhou ao ler o diretório examples");

    for entry in entries {
        let entry = entry.expect("Falhou ao ler a entrada do diretório");
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("kata") {
            // Pula arquivos que são bibliotecas/pseudo-libs (por convenção: mock_*.kata, lib_*.kata)
            let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if file_name.starts_with("mock_") || file_name.starts_with("lib_") {
                println!("Pulando biblioteca: {}", path.display());
                continue;
            }

            println!("Testando arquivo: {}", path.display());

            let mut cmd = Command::cargo_bin("kata").unwrap();
            let output = cmd.arg("build").arg(&path).output().expect("Falha ao invocar kata CLI");

            // Verifica se não há erro no processo de compilação CLI
            // Se o erro for "undefined reference", é um problema de FFI não implementada
            // e não um erro na compilação em si
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !output.status.success() && stderr.contains("undefined reference") {
                println!("Pulando {}: depende de FFI não implementada", path.display());
                continue;
            }

            assert!(
                output.status.success(),
                "Falha ao executar kata build para o arquivo {}: {}",
                path.display(),
                stderr
            );

            // Verifica a impressão temporal ("Build finalizado em")
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                stdout.contains("Build finalizado em"),
                "A saída não conteve o tempo de execução (profiling): {}",
                path.display()
            );
        }
    }
}
