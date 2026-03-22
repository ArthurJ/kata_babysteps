# Especificação Técnica: Generics e Polimorfismo Paramétrico

## Visão Geral
Este documento detalha a implementação de Generics na Kata-Lang. O objetivo é permitir que funções, interfaces e tipos operem sobre tipos abstratos (variáveis de tipo), garantindo segurança estática e performance via monomorfização.

## 1. Identificação de Variáveis de Tipo (Quantificação)
O compilador deve distinguir entre tipos concretos e variáveis genéricas.
- **Convenção:** Identificadores de uma única letra maiúscula (`A`, `T`, `E`) ou nomes em ALL_CAPS que não foram registrados como tipos concretos são tratados como `Type::Var`.
- **Escopo:** Variáveis de tipo são quantificadas universalmente no nível da assinatura da função ou definição da interface.

## 2. Instanciação e Variáveis Frescas
Sempre que uma entidade genérica (função ou interface) é utilizada:
- O compilador deve gerar variáveis de tipo "frescas" (`t_1`, `t_2`, etc.) para evitar colisões entre chamadas independentes.
- Exemplo: `id :: T => T` vira `t_1 => t_1` em um local e `t_2 => t_2` em outro.

## 3. Restrições de Interface (Bounded Polymorphism)
Generics em Kata-Lang não são totalmente "opacos"; eles podem ser restringidos por interfaces.
- **Sintaxe:** `soma :: A B => C with + :: A B => C`
- **Validação:** O Type Checker deve provar que os tipos substitutos para `A` e `B` implementam a interface exigida.

## 4. Hierarquia Transitiva de Interfaces
Para suportar herança de interfaces (Super-Traits), o sistema de tipos deve realizar buscas recursivas no Grafo de Ancestrais.
- **Lógica:** Se `Int implements NUM` e `interface NUM implements ORD`, então `Int` satisfaz `ORD`.
- **Algoritmo:** `satisfies_interface(Type, Interface)` realiza uma busca em largura (BFS) ou profundidade (DFS) no registro de implementações e heranças do `Environment`.

## 5. Unificação e Substituição
O motor Hindley-Milner deve:
- Mapear variáveis de tipo para tipos concretos ou outras variáveis durante a chamada.
- Lançar `TypeMismatch` se uma variável tentar assumir dois tipos incompatíveis no mesmo contexto.

## 6. Monomorfização (Codegen)
Embora a checagem seja genérica, a geração de código criará cópias físicas especializadas para cada combinação de tipos concretos, garantindo custo zero em tempo de execução.
