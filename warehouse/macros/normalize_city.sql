{% macro normalize_city(column_name) %}
    TRIM(
        UPPER(
            -- 1. Take everything before the first comma (removes state/country/extra address)
            SPLIT_PART(
                -- 2. Take everything before the first dash (removes postal codes like Secunderabad-26)
                SPLIT_PART(
                    -- 3. Replace common noise patterns
                    REGEXP_REPLACE(
                        {{ column_name }}, 
                        ' (INDIA|MAHARASHTRA|TELANGANA|KARNATAKA|TAMIL NADU|PUNE|HYDERABAD|BENGALURU)$', 
                        '', 
                        'gi'
                    ),
                    '-', 1
                ),
                ',', 1
            )
        )
    )
{% endmacro %}
