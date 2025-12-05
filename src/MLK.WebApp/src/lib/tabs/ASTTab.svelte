
<script lang="ts">
    import TreeNode from './components/TreeNode.svelte';
    import type { Node } from './components/TreeNode.svelte';

    const treeData: Node = {
        name: "Уровень 1 - Родитель",
        type: "root",
        hidden: false,
        children: [
            {
                name: "Уровень 2 - Ребенок 1",
                hidden: false,
                children: [
                    {
                        name: "Уровень 3 - Ребенок 2",
                        hidden: false,
                        children: [
                            {
                                name: "Уровень 4 - Ребенок 3",
                                hidden: false,
                                children: [
                                    {
                                        name: "Уровень 5 - Ребенок 4",
                                        hidden: false,
                                        children: [
                                            {
                                                name: "Уровень 6 - Ребенок 5",
                                                hidden: false,
                                                children: [
                                                    {
                                                        name: "Уровень 7 - Ребенок 6",
                                                        hidden: false,
                                                        children: [
                                                            {
                                                                name: "Уровень 8 - Ребенок 7",
                                                                hidden: false,
                                                                children: [
                                                                    {
                                                                        name: "Уровень 9 - Ребенок 8",
                                                                        hidden: false,
                                                                        children: [
                                                                            {
                                                                                name: "Уровень 10 - Ребенок 9",
                                                                                hidden: false,
                                                                                children: [
                                                                                    {
                                                                                        name: "Уровень 11 - Ребенок 10",
                                                                                        hidden: false,
                                                                                        children: []
                                                                                    }
                                                                                ]
                                                                            }
                                                                        ]
                                                                    }
                                                                ]
                                                            }
                                                        ]
                                                    }
                                                ]
                                            }
                                        ]
                                    }
                                ]
                            }
                        ]
                    }
                ]
            },
            {
                name: "Уровень 2 - Ребенок 2 (брат)",
                hidden: false,
                children: [
                    {
                        name: "Ветка A",
                        hidden: false,
                        children: [
                            {
                                name: "Лист 1",
                                hidden: false,
                                children: []
                            },
                            {
                                name: "Лист 2",
                                hidden: true,
                                children: [
                                    {
                                        name: "Скрытый элемент",
                                        hidden: false,
                                        children: []
                                    }
                                ]
                            }
                        ]
                    },
                    {
                        name: "Ветка B",
                        hidden: false,
                        children: [
                            {
                                name: "Элемент с типом",
                                type: "special",
                                hidden: false,
                                children: []
                            }
                        ]
                    }
                ]
            },
            {
                name: "Уровень 2 - Ребенок 3",
                hidden: true,
                children: [
                    {
                        name: "Скрытая ветка",
                        hidden: false,
                        children: [
                            {
                                name: "Это не видно пока",
                                hidden: false,
                                children: []
                            }
                        ]
                    }
                ]
            },
            {
                name: "Пустой узел",
                hidden: false,
                children: []
            },
            {
                name: "Большая ветка",
                hidden: false,
                children: [
                    {
                        name: "Подраздел 1",
                        hidden: false,
                        children: [
                            {
                                name: "Элемент 1.1",
                                hidden: false,
                                children: []
                            },
                            {
                                name: "Элемент 1.2",
                                hidden: false,
                                children: [
                                    {
                                        name: "Глубокий элемент",
                                        hidden: false,
                                        children: []
                                    }
                                ]
                            }
                        ]
                    },
                    {
                        name: "Подраздел 2",
                        hidden: false,
                        children: [
                            {
                                name: "Элемент 2.1",
                                hidden: false,
                                children: []
                            },
                            {
                                name: "Элемент 2.2",
                                hidden: false,
                                children: []
                            },
                            {
                                name: "Элемент 2.3",
                                hidden: false,
                                children: []
                            }
                        ]
                    }
                ]
            }
        ]
    };

</script>

<main>
    <ul>
        {#each treeData.children as child}
            <TreeNode node={child} />
        {/each}
    </ul>
</main>