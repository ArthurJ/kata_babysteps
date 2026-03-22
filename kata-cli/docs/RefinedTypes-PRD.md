# Especificação Técnica: Tipos Refinados e Enums Predicados

## Visão Geral
Este documento define a implementação de tipos com restrições lógicas e Enums que particionam o espaço de valores através de predicados.

## 1. Definições
- **Tipo Refinado:** Um tipo base acoplado a um predicado. 
  Ex: `data PositiveInt as (Int, > _ 0)`
- **Enum Predicado:** Um tipo soma onde a variante é escolhida automaticamente por predicados.
  Ex: `enum IMC | Magreza(< _ 18.5) | Normal(<= _ 25.0) | Obesidade`

## 2. Restrições de Predicados (O Alarme de Custo)
A Kata-Lang adota a filosofia de "Liberdade com Consciência" para a performance de predicados. Em vez de uma restrição matemática estrita O(1), o compilador audita o custo da função.
- **Predicados devem ser Funções Puras:** A única restrição absoluta é que o predicado não possua efeitos colaterais (I/O, mutação de estado).
- **Análise de Custo via DAG:** Durante a construção da DAG de dependências, o compilador rastreia a presença de recursões diretas, recursões mútuas ou chamadas a funções de ordem superior inerentemente iterativas (`map`, `fold`).
- **A Diretiva `@heavy_predicate`:** Se o compilador detectar que um predicado possui complexidade temporal superior a O(1) (por usar recursão), ele bloqueará a compilação por segurança. Para aprovar o uso, o programador deve assinar um termo de responsabilidade marcando o tipo ou a variante com a diretiva `@heavy_predicate`, tornando o custo de validação em tempo de execução explícito para toda a equipe.

## 3. Construtores Inteligentes (Smart Constructors)
- Toda definição de tipo refinado gera um construtor que retorna `Result::T::Err`.
- O compilador deve gerar código de máquina otimizado (Jump Tables) para a resolução de variantes em Enums Predicados.

## 4. Degradação e Recuperação de Tipo
- Interações entre Tipos Refinados e Tipos Comuns resultam no tipo comum (degradação).
- A promoção de volta para o tipo refinado exige re-validação via construtor (retornando `Result`) ou o uso de funções de sobrecarga puras que atuam como "provas lógicas".

## 5. Análise de Código Inalcançável
- O Type Checker deve validar a ordem das variantes em Enums Predicados.
- Se a variante A precede a variante B e o predicado de B é um subconjunto lógico de A, o compilador deve emitir um erro de **Unreachable Pattern**.
