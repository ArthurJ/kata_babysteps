# Product Requirements Document (PRD) - Refinamentos Arquiteturais e Templates Funcionais (Fase 9)

## 1. Visão Geral e Contexto
Este documento formaliza as correções arquiteturais e os novos requisitos para a próxima iteração da linguagem Kata. Ele baseia-se em feedback crítico sobre o design do sistema, corrigindo desvios temporários adotados durante a Prova de Conceito (PoC) da Fase 8 e definindo soluções mais elegantes para formatação e introspecção.

Destacam-se neste PRD a correção da semântica de implementação de interfaces, a clarificação do propósito da interface de exibição, a introdução de funções nativas de introspecção em tempo de execução, a definição de um novo sistema explícito para formatação de strings e uma rigorosa auditoria de qualidade no compilador.

## 2. Correções Arquiteturais e Requisitos Funcionais

### 2.1. Declarações `implements` como Contratos Globais
*   **Contexto e Correção:** Durante a PoC da Fase 8, declarações `implements` foram agrupadas dentro de blocos `export` puramente para conveniência na exposição de nativos do Core. Isso foi um erro e um desvio do design da Kata. 
*   **Requisito:** A declaração `implements` é um contrato de módulo global. Ela deve ocorrer de forma livre e independente no escopo global (top-level). 
*   **Impacto no Parser:** O compilador/parser deve aceitar `implements` e `implements auto` soltos no nível do módulo, sem exigir que estejam encapsulados por construções de visibilidade ou exportação.

### 2.2. A Interface `SHOW` (Formatação Visual vs. Serialização)
*   **Contexto e Correção:** Foi gerada a concepção equivocada de que a interface `SHOW` seria necessária para conceder permissão de serialização a um tipo. Na Kata, **todo dado já é nativamente serializável**.
*   **Requisito:** O papel da interface `SHOW` é restrito e focado: ela determina estritamente **como o dado é visualizado/formatado ao ser impresso para o humano ou sistema de log**.
*   **Comportamento:** Quando uma estrutura implementa `SHOW`, ela sobrescreve a representação em texto amigável do dado (ex: saída do `print`), deixando a serialização de dados estrutural (ex: envio pela rede ou armazenamento) totalmente independente e inalterada.

### 2.3. Funções Nativas de Introspecção em Tempo de Execução
*   **Requisito:** Introdução de funções *built-in* normais da linguagem, disponíveis globalmente para uso regular em tempo de execução, fornecendo introspecção nominal de primeira classe.
*   **Novas Funções Nativas:**
    *   `type(obj)`: Retorna a representação estrita do tipo do objeto (substitui a nomenclatura teórica `type_name`).
    *   `fields(obj)`: Retorna os campos/chaves da estrutura em tempo de execução, habilitando iteração avançada e validações diretas no código comum.

### 2.4. Interpolação de Strings via Templates Funcionais
*   **Contexto:** O design anterior cogitava a inclusão de "f-strings" mágicas (ex: `"O valor é {x}"`), o que exigiria um Lexer com estados complexos para fazer parsing de código arbitrário dentro de tokens de string, injetando comportamentos impuros.
*   **Requisito:** Adoção de **Templates Funcionais** (inspirado na abordagem elegante do `str.format()` do Python).
*   **Comportamento:** 
    *   As Strings mantêm sua pureza lexical original. Não há avaliação de código embutido na string durante a análise léxica.
    *   O desenvolvedor passará uma string estática (atuando como Template) para uma função de formatação, passando os valores a serem substituídos como argumentos independentes.
    *   *Exemplo conceitual:* Chamadas explícitas de função em vez de sintaxe mágica. A lógica de substituição e interpolação ocorre de forma rastreável, mantendo a simplicidade do analisador léxico.

## 3. Impactos Não Funcionais e Regras de Engenharia

### 3.1. Auditoria Anti-Hardcode (Nova Regra de Engenharia Estrita)
*   **Contexto e Problema:** Durante as PoCs anteriores, notou-se que o compilador acumulou "hacks" e lógicas chumbadas (hardcoded) focadas em fazer testes específicos passarem.
*   **Requisito Obrigatório:** **Todo o código-fonte do compilador atual em Rust deve passar por uma auditoria rígida.**
*   **Ação:** Buscar e remover absolutamente quaisquer "hacks", atalhos ou lógicas no compilador que dependam de nomes de arquivos, nomes de variáveis, funções ou valores literais específicos dos scripts de teste. 
*   **Meta:** O compilador deve processar a semântica da linguagem de forma puramente genérica. A linguagem tem que processar e entender as estruturas sem conhecer os scripts de antemão ou injetar comportamentos sob medida para exemplos.

### 3.2. Simplicidade e Transparência
*   **Simplicidade do Compilador:** A decisão de evitar f-strings mágicas garante que o Lexer da Kata se mantenha rápido, simples e fácil de manter, sem necessidade de gerenciar pilhas de estados aninhados para analisar expressões dentro de strings.
*   **Transparência:** A abordagem de Templates Funcionais e a independência da cláusula `implements` tornam a intenção do código explícita para o desenvolvedor, alinhando-se aos princípios fundamentais da linguagem.
*   **Separação de Preocupações:** A clara distinção entre serialização inerente e visualização orientada pela interface `SHOW` elimina ambiguidades no tratamento de dados.

## 4. Próximos Passos
1.  **Executar a Auditoria Anti-Hardcode:** Varrer o repositório em Rust e remover todos os acoplamentos e hacks de exemplo inseridos em fases anteriores.
2.  **Atualizar a Gramática:** Modificar o Parser para desvincular o `implements` de blocos de exportação, se estiver vinvulado.
3.  **Implementar Primitivas Nativas:** Adicionar `type` e `fields` como *built-ins* globais disponíveis no tempo de execução.
4.  **Projetar Templates Funcionais:** Definir a assinatura oficial da função de substituição de strings na biblioteca padrão.
